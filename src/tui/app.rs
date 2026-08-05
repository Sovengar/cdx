use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Matcher, Config, Utf32Str};
use ratatui::text::Text;
use ratatui::widgets::ListState;

use crate::config;
use crate::preview::PreviewEntry;
use crate::walker::DirEntryItem;
use crate::zoxide;

use super::grep;

struct SearchResult {
    generation: u64,
    root: PathBuf,
    entries: Vec<DirEntryItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Find,
    Search,
    Grep,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Focus {
    List,
    Preview,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Popup {
    ToolSelector,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ExitAction {
    OutputPath(PathBuf),
    SpawnTool { command: String, path: PathBuf },
    JustExit,
}

pub struct App {
    pub current_dir: PathBuf,
    pub items: Vec<DirEntryItem>,
    pub filtered_indices: Vec<usize>,

    pub mode: Mode,
    pub focus: Focus,
    pub popup: Option<Popup>,
    pub popup_index: usize,
    pub show_dotfiles: bool,
    pub show_winhidden: bool,

    pub list_state: ListState,
    pub query: String,
    pub cursor_pos: usize,

    pub popup_width_pct: u16,
    pub popup_height_pct: u16,

    pub should_quit: bool,
    pub exit_action: ExitAction,

    pub zoxide_cache: Vec<PathBuf>,
    zoxide_initial_visible: bool,
    pub preview_text: Text<'static>,
    pub preview_contents: Text<'static>,
    pub preview_dirty: bool,
    pub preview_scroll: u64,
    pub preview_entries: Vec<PreviewEntry>,
    pub preview_selection: usize,

    pub matcher: Matcher,
    pub scratch: Vec<char>,

    pub grep_pending: bool,
    pub grep_results: Vec<grep::GrepMatch>,
    pub grep_search_root: PathBuf,
    pub grep_debounce_ms: u64,

    pub find_debounce_ms: u64,
    pub find_cache: Vec<DirEntryItem>,
    pub find_cache_root: PathBuf,
    pub find_cache_loaded: bool,
    pub find_deadline: Option<Instant>,
    pub search_generation: u64,
    pub search_cancel: Option<Arc<AtomicBool>>,
    search_results_tx: mpsc::Sender<SearchResult>,
    search_results: mpsc::Receiver<SearchResult>,

    pub tick: u64,
    pub last_esc_time: Option<Instant>,
    pub spawn_pending: Option<(String, PathBuf)>,
}

impl App {
    pub fn new(initial_query: Option<String>) -> anyhow::Result<Self> {
        let current_dir = std::env::current_dir()?;
        let cfg = config::get();
        let zoxide_cache = zoxide::get_list();

        let query = initial_query.unwrap_or_default();
        let cursor_pos = query.len();
        let zoxide_initial_visible = query.is_empty();

        let (search_tx, search_results) = mpsc::channel();
        let find_deadline = if query.is_empty() {
            None
        } else {
            Some(Instant::now() + Duration::from_millis(cfg.find_debounce_ms))
        };

        Ok(Self {
            current_dir,
            items: Vec::new(),
            filtered_indices: Vec::new(),

            mode: Mode::Find,
            focus: Focus::List,
            popup: None,
            popup_index: 0,
            show_dotfiles: cfg.show_dotfiles,
            show_winhidden: cfg.show_winhidden,

            list_state: ListState::default(),
            query,
            cursor_pos,

            popup_width_pct: cfg.popup_width_pct,
            popup_height_pct: cfg.popup_height_pct,

            should_quit: false,
            exit_action: ExitAction::JustExit,

            zoxide_cache,
            zoxide_initial_visible,
            preview_text: Text::default(),
            preview_contents: Text::default(),
            preview_dirty: false,
            preview_scroll: 0,
            preview_entries: Vec::new(),
            preview_selection: 0,

            matcher: Matcher::new(Config::DEFAULT.match_paths()),
            scratch: Vec::new(),

            grep_pending: false,
            grep_results: Vec::new(),
            grep_search_root: dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")),
            grep_debounce_ms: cfg.grep_debounce_ms,

            find_debounce_ms: cfg.find_debounce_ms,
            find_cache: Vec::new(),
            find_cache_root: PathBuf::new(),
            find_cache_loaded: false,
            find_deadline,
            search_generation: 0,
            search_cancel: None,
            search_results_tx: search_tx,
            search_results,

            tick: 0,
            last_esc_time: None,
            spawn_pending: None,
        })
    }

    pub fn refresh_items(&mut self) {
        match self.mode {
            Mode::Find => {
                if self.query.is_empty() {
                    self.load_find_items();
                }
            }
            Mode::Search => {
                self.items = crate::walker::list_files(
                    &self.current_dir,
                    self.show_dotfiles,
                    self.show_winhidden,
                );
                self.apply_query();
            }
            Mode::Grep => {
                return;
            }
        }
    }

    fn load_find_items(&mut self) {
        let walker_items = crate::walker::list_dirs(
            &self.current_dir,
            self.show_dotfiles,
            self.show_winhidden,
        );
        self.items = if self.zoxide_initial_visible {
            self.zoxide_initial_visible = false;
            self.merge_zoxide(walker_items)
        } else {
            walker_items
        };
        self.filtered_indices = (0..self.items.len()).collect();
        if !self.filtered_indices.is_empty() {
            self.list_state.select(Some(0));
            self.preview_dirty = true;
        }
    }

    fn invalidate_find_cache(&mut self) {
        self.search_generation = self.search_generation.wrapping_add(1);
        self.cancel_search();
        self.find_cache.clear();
        self.find_cache_root.clear();
        self.find_cache_loaded = false;
        self.find_deadline = None;
    }

    pub fn apply_query(&mut self) {
        if self.mode == Mode::Grep {
            self.cancel_search();
            self.find_deadline = None;
            if !self.query.is_empty() {
                self.grep_pending = true;
            }
            return;
        }

        self.search_generation = self.search_generation.wrapping_add(1);

        if self.query.is_empty() {
            self.cancel_search();
            self.find_deadline = None;
            if self.mode == Mode::Find {
                self.load_find_items();
            } else {
                self.filtered_indices = (0..self.items.len()).collect();
            }
            if !self.filtered_indices.is_empty() {
                self.list_state.select(Some(0));
            }
            return;
        }

        self.cancel_search();
        self.find_deadline = Some(
            Instant::now() + Duration::from_millis(self.find_debounce_ms),
        );
    }

    pub fn start_find_search(&mut self) {
        let Some(deadline) = self.find_deadline else {
            return;
        };
        if Instant::now() < deadline {
            return;
        }
        self.find_deadline = None;

        if self.query.is_empty() {
            return;
        }

        if self.mode == Mode::Find {
            if self.find_cache_root != self.current_dir || !self.find_cache_loaded {
                self.spawn_directory_search();
            } else {
                self.filter_find_cache();
            }
            return;
        }

        // Search mode: score current items
        let pattern = Pattern::parse(
            &self.query,
            CaseMatching::Ignore,
            Normalization::Smart,
        );

        let mut scored: Vec<(usize, u32)> = self
            .items
            .iter()
            .enumerate()
            .filter_map(|(i, item)| {
                self.scratch.clear();
                let haystack = Utf32Str::new(item.display.as_str(), &mut self.scratch);
                pattern.score(haystack, &mut self.matcher).map(|s| (i, s))
            })
            .collect();

        scored.sort_by(|a, b| b.1.cmp(&a.1));
        self.filtered_indices = scored.into_iter().map(|(i, _)| i).collect();

        if !self.filtered_indices.is_empty() {
            self.list_state.select(Some(0));
        }
    }

    fn spawn_directory_search(&mut self) {
        let root = self.current_dir.clone();
        let show_dotfiles = self.show_dotfiles;
        let show_winhidden = self.show_winhidden;
        let generation = self.search_generation;
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = Arc::clone(&cancel);
        let sender = self.search_results_tx.clone();

        self.search_cancel = Some(cancel);
        std::thread::spawn(move || {
            let entries = crate::walker::recursive_dir_search(
                &root,
                show_dotfiles,
                show_winhidden,
                &worker_cancel,
            );
            if !worker_cancel.load(Ordering::Relaxed) {
                let _ = sender.send(SearchResult {
                    generation,
                    root,
                    entries,
                });
            }
        });
    }

    fn filter_find_cache(&mut self) {
        let pattern = Pattern::parse(
            &self.query,
            CaseMatching::Ignore,
            Normalization::Smart,
        );

        let mut scored: Vec<(usize, u32)> = self
            .find_cache
            .iter()
            .enumerate()
            .filter_map(|(i, item)| {
                self.scratch.clear();
                let haystack = Utf32Str::new(item.display.as_str(), &mut self.scratch);
                pattern.score(haystack, &mut self.matcher).map(|s| (i, s))
            })
            .collect();

        scored.sort_by(|a, b| b.1.cmp(&a.1));
        scored.truncate(200);

        let top_indices: Vec<usize> = scored.into_iter().map(|(i, _)| i).collect();
        self.items = top_indices
            .iter()
            .map(|&i| self.find_cache[i].clone())
            .collect();
        self.filtered_indices = (0..self.items.len()).collect();

        if !self.filtered_indices.is_empty() {
            self.list_state.select(Some(0));
        }
    }

    pub fn receive_search_results(&mut self) {
        while let Ok(result) = self.search_results.try_recv() {
            if self.mode != Mode::Find
                || self.query.is_empty()
                || result.generation != self.search_generation
                || result.root != self.current_dir
            {
                continue;
            }

            self.find_cache = result.entries;
            self.find_cache_root = result.root;
            self.find_cache_loaded = true;
            self.search_cancel = None;
            self.filter_find_cache();
        }
    }

    pub fn cancel_search(&mut self) {
        if let Some(cancel) = self.search_cancel.take() {
            cancel.store(true, Ordering::Relaxed);
        }
    }

    pub fn search_poll_timeout(&self) -> Duration {
        let base = Duration::from_millis(50);
        self.find_deadline
            .map(|deadline| deadline.saturating_duration_since(Instant::now()).min(base))
            .unwrap_or(base)
    }

    pub fn search_due(&self) -> bool {
        self.find_deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
    }

    fn reset_preview(&mut self) {
        self.preview_text = Text::default();
        self.preview_contents = Text::default();
        self.preview_entries.clear();
        self.preview_selection = 0;
        self.preview_scroll = 0;
    }

    fn navigate_to(&mut self, directory: PathBuf) {
        self.current_dir = directory;
        if self.mode == Mode::Grep {
            self.grep_search_root = self.current_dir.clone();
        }

        self.reset_preview();
        self.invalidate_find_cache();
        self.items.clear();
        self.filtered_indices.clear();
        self.list_state.select(None);
        self.grep_results.clear();
        self.grep_pending = false;

        self.refresh_items();
        self.apply_query();
        self.preview_dirty = true;
    }

    pub fn handle_enter(&mut self) {
        let selected = self.list_state.selected();
        let idx = match selected.and_then(|s| self.filtered_indices.get(s)) {
            Some(&i) => i,
            None => return,
        };
        let item = match self.items.get(idx) {
            Some(item) => item,
            None => return,
        };

        match self.mode {
            Mode::Find => {
                if item.is_dir && item.full_path.is_dir() {
                    self.current_dir = item.full_path.clone();
                    self.query.clear();
                    self.cursor_pos = 0;
                    self.reset_preview();
                    self.invalidate_find_cache();
                    self.refresh_items();
                    self.preview_dirty = true;
                }
            }
            Mode::Search => {
                if let Some(parent) = item.full_path.parent() {
                    if parent.exists() {
                        self.current_dir = parent.to_path_buf();
                        self.query.clear();
                        self.cursor_pos = 0;
                        self.reset_preview();
                        self.mode = Mode::Find;
                        self.invalidate_find_cache();
                        self.refresh_items();
                        self.preview_dirty = true;
                    }
                }
            }
            Mode::Grep => {
                if let Some(parent) = item.full_path.parent() {
                    if parent.exists() {
                        self.current_dir = parent.to_path_buf();
                        self.query.clear();
                        self.cursor_pos = 0;
                        self.reset_preview();
                        self.mode = Mode::Find;
                        self.invalidate_find_cache();
                        self.refresh_items();
                        self.preview_dirty = true;
                    }
                }
            }
        }
    }

    pub fn handle_esc(&mut self) {
        if let Some(last) = self.last_esc_time {
            if last.elapsed() < Duration::from_millis(300) {
                self.last_esc_time = None;
                if let Some(home) = dirs::home_dir() {
                    self.navigate_to(home);
                }
                return;
            }
        }
        self.last_esc_time = Some(Instant::now());
        if let Some(parent) = self.current_dir.parent() {
            if parent.as_os_str().is_empty() {
                self.should_quit = true;
            } else {
                let parent = parent.to_path_buf();
                if parent.exists() {
                    self.navigate_to(parent);
                }
            }
        } else {
            self.should_quit = true;
        }
    }

    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        if self.popup.is_some() {
            self.handle_popup_key(key);
            return;
        }

        if self.handle_config_key(&key) {
            return;
        }

        use crossterm::event::KeyCode;

        match self.focus {
            Focus::List => {
                match key.code {
                    KeyCode::Up => self.list_nav_up(),
                    KeyCode::Down => self.list_nav_down(),
                    KeyCode::PageUp => self.preview_scroll = self.preview_scroll.saturating_sub(10),
                    KeyCode::PageDown => self.preview_scroll = self.preview_scroll.saturating_add(10),
                    KeyCode::Enter => self.handle_enter(),
                    _ => self.handle_list_key(key),
                }
            }
            Focus::Preview => self.handle_preview_key(key),
        }
    }

    fn list_nav_up(&mut self) {
        let i = self.list_state.selected().unwrap_or(0);
        if i > 0 {
            self.list_state.select(Some(i - 1));
            self.preview_dirty = true;
        }
    }

    fn list_nav_down(&mut self) {
        let i = self.list_state.selected().unwrap_or(0);
        if i + 1 < self.filtered_indices.len() {
            self.list_state.select(Some(i + 1));
            self.preview_dirty = true;
        }
    }

    fn handle_popup_key(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;

        match self.popup {
            Some(Popup::ToolSelector) => {
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        if self.popup_index > 0 {
                            self.popup_index -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        let tool_count = crate::config::get().tool_selector.len();
                        if tool_count > 0 && self.popup_index < tool_count - 1 {
                            self.popup_index += 1;
                        }
                    }
                    KeyCode::Enter => {
                        self.execute_tool_from_popup();
                    }
                    KeyCode::Esc | KeyCode::Char('q') => {
                        self.popup = None;
                    }
                    _ => {}
                }
            }
            None => {}
        }
    }

    fn execute_tool_from_popup(&mut self) {
        let dir = self.get_selected_dir();
        let tools = &crate::config::get().tool_selector;
        let tool = match tools.get(self.popup_index) {
            Some(t) => t,
            None => return,
        };
        self.popup = None;
        if let Some(path) = dir {
            self.spawn_pending = Some((tool.command.clone(), path));
        }
    }

    fn get_selected_dir(&self) -> Option<PathBuf> {
        let idx = self.list_state.selected()?;
        let item_idx = self.filtered_indices.get(idx)?;
        let item = self.items.get(*item_idx)?;
        if item.is_dir {
            Some(item.full_path.clone())
        } else {
            None
        }
    }
    fn handle_preview_key(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::{KeyCode, KeyModifiers};

        match key.code {
            KeyCode::Up => {
                if self.preview_selection > 0 {
                    self.preview_selection -= 1;
                }
            }
            KeyCode::Down => {
                if self.preview_selection + 1 < self.preview_entries.len() {
                    self.preview_selection += 1;
                }
            }
            KeyCode::Enter if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if let Some(idx) = self.list_state.selected() {
                    if let Some(&item_idx) = self.filtered_indices.get(idx) {
                        if let Some(item) = self.items.get(item_idx) {
                            if item.is_dir {
                                self.popup = Some(Popup::ToolSelector);
                                self.popup_index = 0;
                            }
                        }
                    }
                }
            }
            KeyCode::Enter => {
                if let Some(entry) = self.preview_entries.get(self.preview_selection) {
                    if entry.is_dir && entry.full_path.is_dir() {
                        self.current_dir = entry.full_path.clone();
                        self.query.clear();
                        self.cursor_pos = 0;
                        self.reset_preview();
                        self.focus = Focus::List;
                        self.invalidate_find_cache();
                        self.refresh_items();
                        self.preview_dirty = true;
                    }
                }
            }
            KeyCode::Left | KeyCode::Esc => {
                self.focus = Focus::List;
            }
            KeyCode::Tab => {
                self.focus = Focus::List;
                self.switch_mode();
            }
            KeyCode::Char(c) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.handle_config_key(&key);
                } else {
                    self.query.insert(self.cursor_pos, c);
                    self.cursor_pos += 1;
                    self.apply_query();
                }
            }
            _ => {}
        }
    }

    fn handle_list_key(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::{KeyCode, KeyModifiers};

        match key.code {
            KeyCode::Left => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                }
            }
            KeyCode::Right => {
                let has_preview = !self.preview_text.lines.is_empty()
                    || !self.preview_contents.lines.is_empty();
                if has_preview {
                    self.focus = Focus::Preview;
                } else if self.cursor_pos < self.query.len() {
                    self.cursor_pos += 1;
                }
            }
            KeyCode::Home => {
                self.cursor_pos = 0;
            }
            KeyCode::End => {
                self.cursor_pos = self.query.len();
            }
            KeyCode::Backspace => {
                if self.cursor_pos > 0 {
                    self.query.remove(self.cursor_pos - 1);
                    self.cursor_pos -= 1;
                    self.apply_query();
                }
            }
            KeyCode::Char(c) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.handle_config_key(&key);
                } else {
                    self.query.insert(self.cursor_pos, c);
                    self.cursor_pos += 1;
                    self.apply_query();
                }
            }
            KeyCode::Esc => {
                self.handle_esc();
            }
            _ => {}
        }
    }

    fn handle_config_key(&mut self, key: &crossterm::event::KeyEvent) -> bool {
        use crate::config;
        let action = config::get().keys.match_action(key);
        match action {
            Some("quit") => { self.should_quit = true; true }
            Some("toggle_dotfiles") => {
                self.show_dotfiles = !self.show_dotfiles;
                self.invalidate_find_cache();
                self.refresh_items();
                self.apply_query();
                true
            }
            Some("toggle_winhidden") => {
                self.invalidate_find_cache();
                self.toggle_winhidden();
                self.apply_query();
                true
            }
            Some("open_settings") => {
                let cfg_path = dirs::home_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from("."))
                    .join(".config")
                    .join("cdx")
                    .join("config.toml");
                self.spawn_pending = Some(("nvim".to_string(), cfg_path));
                true
            }
            Some("switch_mode") => { self.switch_mode(); true }
            Some("open_explorer") => {
                if let Some(idx) = self.list_state.selected() {
                    if let Some(&item_idx) = self.filtered_indices.get(idx) {
                        if let Some(item) = self.items.get(item_idx) {
                            if item.is_dir {
                                self.popup = Some(Popup::ToolSelector);
                                self.popup_index = 0;
                            }
                        }
                    }
                }
                true
            }
            Some("go_home") => {
                if let Some(home) = dirs::home_dir() {
                    self.current_dir = home;
                    self.query.clear();
                    self.cursor_pos = 0;
                    self.reset_preview();
                    self.invalidate_find_cache();
                    self.refresh_items();
                    self.preview_dirty = true;
                }
                true
            }
            _ => false,
        }
    }

    pub fn switch_mode(&mut self) {
        self.search_generation = self.search_generation.wrapping_add(1);
        self.cancel_search();
        self.find_deadline = None;
        self.mode = match self.mode {
            Mode::Find => Mode::Search,
            Mode::Search => Mode::Grep,
            Mode::Grep => Mode::Find,
        };

        if self.mode == Mode::Grep {
            let was_hidden = self.show_winhidden;
            self.show_winhidden = false;
            self.grep_search_root = self.current_dir.clone();

            if was_hidden {
                eprintln!("[cdx] Grep mode: WinHidden auto-disabled (Ctrl+W to re-enable)");
            }

            if !self.query.is_empty() {
                self.grep_pending = true;
            }
        }

        self.refresh_items();
        self.apply_query();
        self.preview_dirty = true;
    }

    pub fn toggle_winhidden(&mut self) {
        self.show_winhidden = !self.show_winhidden;
        if self.mode == Mode::Grep && !self.query.is_empty() {
            self.grep_pending = true;
        }
        self.refresh_items();
    }

    pub fn run_grep_search(&mut self) {
        self.grep_pending = false;

        if self.query.is_empty() {
            self.grep_results.clear();
            self.items.clear();
            self.filtered_indices.clear();
            return;
        }

        let root = self.grep_search_root.clone();
        let output = grep::run_rg_command(&root, &self.query, self.show_winhidden, self.show_dotfiles, true);

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Detect if rg supports --json: first non-empty line should start with '{'
        let is_json = stdout.lines().find(|l| !l.is_empty()).is_some_and(|l| l.starts_with('{'));

        if is_json {
            self.grep_results = grep::parse_rg_json(&stdout);
        } else {
            // Fallback: rg < 13.0.0 doesn't support --json, re-run with --vimgrep
            let fallback = grep::run_rg_command(&root, &self.query, self.show_winhidden, self.show_dotfiles, false);
            let fallback_stdout = String::from_utf8_lossy(&fallback.stdout);
            self.grep_results = grep::parse_rg_vimgrep(&fallback_stdout);
        }

        self.items = self
            .grep_results
            .iter()
            .map(|m| {
                let full_path = if std::path::Path::new(&m.file_path).is_absolute() {
                    std::path::PathBuf::from(&m.file_path)
                } else {
                    root.join(&m.file_path)
                };
                let display_path = if let Ok(rel) = std::path::Path::new(&m.file_path).strip_prefix(&root) {
                    rel.to_string_lossy().replace('\\', "/")
                } else {
                    m.file_path.clone()
                };
                DirEntryItem {
                    display: format!("{}:{}", display_path, m.line_number),
                    rel_path: full_path.to_string_lossy().replace('\\', "/"),
                    full_path,
                    is_zoxide: false,
                    is_dir: false,
                }
            })
            .collect();

        self.filtered_indices = (0..self.items.len()).collect();
        if !self.filtered_indices.is_empty() {
            self.list_state.select(Some(0));
        }
    }

    fn merge_zoxide(&mut self, mut walker_items: Vec<DirEntryItem>) -> Vec<DirEntryItem> {
        let home = dirs::home_dir();
        let home_str = home.as_ref().map(|h| h.to_string_lossy().replace('\\', "/"));
        let home_lower = home_str.as_ref().map(|s| s.to_lowercase());
        let mut zoxide_items: Vec<DirEntryItem> = Vec::new();
        let limit = config::get().zoxide_limit;

        for zpath in &self.zoxide_cache {
            let full_str = zpath.to_string_lossy().replace('\\', "/");
            if let Some(ref hl) = home_lower {
                if !full_str.to_lowercase().starts_with(hl) {
                    continue;
                }
            }
            let exists = walker_items.iter().any(|w| w.full_path == *zpath);
            let is_current = *zpath == self.current_dir;
            if exists || is_current {
                continue;
            }
            let display = if let Some(ref hs) = home_str {
                if full_str.starts_with(hs.as_str()) {
                    format!("~{}", &full_str[hs.len()..])
                } else {
                    full_str.clone()
                }
            } else {
                full_str.clone()
            };
            zoxide_items.push(DirEntryItem {
                display,
                rel_path: zpath.to_string_lossy().replace('\\', "/"),
                full_path: zpath.clone(),
                is_zoxide: true,
                is_dir: true,
            });
            if zoxide_items.len() >= limit {
                break;
            }
        }

        walker_items.retain(|w| {
            !zoxide_items.iter().any(|z| z.full_path == w.full_path)
        });

        zoxide_items.extend(walker_items);
        zoxide_items
    }
}
