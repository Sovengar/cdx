use std::path::PathBuf;
use std::time::{Duration, Instant};

use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Matcher, Config, Utf32Str};
use ratatui::text::Text;
use ratatui::widgets::ListState;

use crate::config;
use crate::preview::PreviewEntry;
use crate::walker::DirEntryItem;
use crate::zoxide;

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

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ExitAction {
    OutputPath(PathBuf),
    SpawnYazi(PathBuf),
    JustExit,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct GrepMatch {
    pub file_path: String,
    pub line_number: u32,
    pub match_text: String,
}

pub struct App {
    pub current_dir: PathBuf,
    pub items: Vec<DirEntryItem>,
    pub filtered_indices: Vec<usize>,

    pub mode: Mode,
    pub focus: Focus,
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
    pub preview_text: Text<'static>,
    pub preview_contents: Text<'static>,
    pub preview_dirty: bool,
    pub preview_scroll: u64,
    pub preview_entries: Vec<PreviewEntry>,
    pub preview_selection: usize,

    pub matcher: Matcher,
    pub scratch: Vec<char>,

    pub grep_pending: bool,
    pub grep_results: Vec<GrepMatch>,
    pub grep_search_root: PathBuf,
    pub grep_debounce_ms: u64,

    pub find_pending: bool,
    pub find_debounce_ms: u64,
    pub find_cache: Vec<DirEntryItem>,
    pub find_cache_root: PathBuf,

    pub tick: u64,
    pub last_esc_time: Option<Instant>,
    pub edit_pending: bool,
}

impl App {
    pub fn new(initial_query: Option<String>) -> anyhow::Result<Self> {
        let current_dir = std::env::current_dir()?;
        let cfg = config::get();
        let zoxide_cache = zoxide::get_list();

        let query = initial_query.unwrap_or_default();
        let cursor_pos = query.len();

        Ok(Self {
            current_dir,
            items: Vec::new(),
            filtered_indices: Vec::new(),

            mode: Mode::Find,
            focus: Focus::List,
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

            find_pending: false,
            find_debounce_ms: cfg.find_debounce_ms,
            find_cache: Vec::new(),
            find_cache_root: PathBuf::new(),

            tick: 0,
            last_esc_time: None,
            edit_pending: false,
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
        self.items = self.merge_zoxide(walker_items);
        self.filtered_indices = (0..self.items.len()).collect();
        if !self.filtered_indices.is_empty() {
            self.list_state.select(Some(0));
            self.preview_dirty = true;
        }
    }

    fn invalidate_find_cache(&mut self) {
        self.find_cache.clear();
        self.find_cache_root.clear();
    }

    pub fn apply_query(&mut self) {
        if self.mode == Mode::Grep {
            if !self.query.is_empty() {
                self.grep_pending = true;
            }
            return;
        }

        if self.query.is_empty() {
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

        self.find_pending = true;
    }

    pub fn run_find_search(&mut self) {
        self.find_pending = false;

        if self.query.is_empty() {
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

        if self.mode == Mode::Find {
            if self.find_cache_root != self.current_dir || self.find_cache.is_empty() {
                self.find_cache = crate::walker::recursive_dir_search(
                    &self.current_dir,
                    self.show_dotfiles,
                    self.show_winhidden,
                );
                self.find_cache_root = self.current_dir.clone();
            }

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
            self.items = top_indices.iter().map(|&i| self.find_cache[i].clone()).collect();
            self.filtered_indices = (0..self.items.len()).collect();

            if !self.filtered_indices.is_empty() {
                self.list_state.select(Some(0));
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

    fn reset_preview(&mut self) {
        self.preview_text = Text::default();
        self.preview_contents = Text::default();
        self.preview_entries.clear();
        self.preview_selection = 0;
        self.preview_scroll = 0;
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
                // Preview panel shows file content; Enter is a no-op for now
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
                    self.current_dir = home;
                    self.query.clear();
                    self.cursor_pos = 0;
                    self.reset_preview();
                    self.invalidate_find_cache();
                    self.refresh_items();
                    self.preview_dirty = true;
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
                    self.current_dir = parent;
                    self.reset_preview();
                    self.invalidate_find_cache();
                    self.refresh_items();
                    self.preview_dirty = true;
                    self.apply_query();
                }
            }
        } else {
            self.should_quit = true;
        }
    }

    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
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
                                self.exit_action = ExitAction::SpawnYazi(item.full_path.clone());
                                self.should_quit = true;
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
            Some("open_settings") => { self.edit_pending = true; true }
            Some("switch_mode") => { self.switch_mode(); true }
            Some("open_explorer") => {
                if let Some(idx) = self.list_state.selected() {
                    if let Some(&item_idx) = self.filtered_indices.get(idx) {
                        if let Some(item) = self.items.get(item_idx) {
                            if item.is_dir {
                                self.exit_action = ExitAction::SpawnYazi(item.full_path.clone());
                                self.should_quit = true;
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
        self.mode = match self.mode {
            Mode::Find => Mode::Search,
            Mode::Search => Mode::Grep,
            Mode::Grep => Mode::Find,
        };

        if self.mode == Mode::Grep {
            let was_hidden = self.show_winhidden;
            self.show_winhidden = false;
            self.grep_search_root = self.clamp_search_root();

            if was_hidden {
                eprintln!("[cdx] Grep mode: WinHidden auto-disabled (Ctrl+W to re-enable)");
            }

            if !self.query.is_empty() {
                self.grep_pending = true;
            }
        }

        self.refresh_items();
        self.apply_query();
    }

    pub fn toggle_winhidden(&mut self) {
        self.show_winhidden = !self.show_winhidden;
        if self.mode == Mode::Grep && !self.query.is_empty() {
            self.grep_pending = true;
        }
        self.refresh_items();
    }

    fn clamp_search_root(&self) -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        if self.current_dir.starts_with(&home) {
            self.current_dir.clone()
        } else {
            home
        }
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

        let mut cmd = std::process::Command::new("rg");
        cmd.args(["--vimgrep", "--smart-case"]);
        cmd.args(["--max-depth", &config::get().grep_max_depth.to_string()]);

        for d in config::get().exclude_dirs.iter() {
            cmd.args(["--glob", &format!("!{}", d)]);
        }
        for p in config::get().exclude_path_globs.iter() {
            cmd.args(["--glob", &format!("!{}", p)]);
        }
        if !self.show_winhidden {
            for d in config::get().exclude_win_dirs.iter() {
                cmd.args(["--glob", &format!("!{}", d)]);
            }
        }
        if !self.show_dotfiles {
            cmd.args(["--glob", "!.*"]);
        }

        cmd.arg(&self.query).arg(&root);

        let output = match cmd.output() {
            Ok(o) => o,
            Err(_) => {
                self.grep_results.clear();
                return;
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        self.grep_results = stdout
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.splitn(4, ':').collect();
                if parts.len() < 4 {
                    return None;
                }
                Some(GrepMatch {
                    file_path: parts[0].to_string(),
                    line_number: parts[1].parse().unwrap_or(0),
                    match_text: parts[3].to_string(),
                })
            })
            .collect();

        self.items = self
            .grep_results
            .iter()
            .map(|m| DirEntryItem {
                display: format!("{}:{}", m.file_path, m.line_number),
                rel_path: m.file_path.clone(),
                full_path: root.join(&m.file_path),
                is_zoxide: false,
                is_dir: false,
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
