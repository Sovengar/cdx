use std::path::PathBuf;

use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Matcher, Config, Utf32Str};
use ratatui::text::Text;
use ratatui::widgets::ListState;

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
}

impl App {
    pub fn new(initial_query: Option<String>) -> anyhow::Result<Self> {
        let current_dir = std::env::current_dir()?;
        let zoxide_cache = zoxide::get_list();

        let query = initial_query.unwrap_or_default();
        let cursor_pos = query.len();

        Ok(Self {
            current_dir,
            items: Vec::new(),
            filtered_indices: Vec::new(),

            mode: Mode::Find,
            focus: Focus::List,
            show_dotfiles: false,
            show_winhidden: false,

            list_state: ListState::default(),
            query,
            cursor_pos,

            popup_width_pct: 80,
            popup_height_pct: 80,

            should_quit: false,
            exit_action: ExitAction::JustExit,

            zoxide_cache,
            preview_text: Text::default(),
            preview_contents: Text::default(),
            preview_dirty: false,
            preview_scroll: 0,

            matcher: Matcher::new(Config::DEFAULT.match_paths()),
            scratch: Vec::new(),

            grep_pending: false,
            grep_results: Vec::new(),
            grep_search_root: dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")),
            grep_debounce_ms: 300,

            find_pending: false,
            find_debounce_ms: 150,
            find_cache: Vec::new(),
            find_cache_root: PathBuf::new(),

            tick: 0,
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

        if self.mode == Mode::Find {
            self.find_pending = true;
            return;
        }

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

    pub fn run_find_search(&mut self) {
        self.find_pending = false;

        if self.query.is_empty() {
            self.load_find_items();
            return;
        }

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
                    self.preview_text = Text::default();
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
                        self.preview_text = Text::default();
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
        if let Some(parent) = self.current_dir.parent() {
            if parent.as_os_str().is_empty() {
                self.should_quit = true;
            } else {
                let parent = parent.to_path_buf();
                if parent.exists() {
                    self.current_dir = parent;
                    self.preview_text = Text::default();
                    self.invalidate_find_cache();
                    self.refresh_items();
                }
            }
        } else {
            self.should_quit = true;
        }
    }

    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        match self.focus {
            Focus::Preview => self.handle_preview_key(key),
            Focus::List => self.handle_list_key(key),
        }
    }

    fn handle_preview_key(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Left | KeyCode::Esc => {
                self.focus = Focus::List;
            }
            KeyCode::Up => {
                self.preview_scroll = self.preview_scroll.saturating_sub(1);
            }
            KeyCode::Down => {
                self.preview_scroll = self.preview_scroll.saturating_add(1);
            }
            KeyCode::PageUp => {
                self.preview_scroll = self.preview_scroll.saturating_sub(10);
            }
            KeyCode::PageDown => {
                self.preview_scroll = self.preview_scroll.saturating_add(10);
            }
            KeyCode::Enter => {
                self.handle_enter();
            }
            KeyCode::Char(c) => {
                if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                    self.handle_global(key);
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
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Up => {
                let i = self.list_state.selected().unwrap_or(0);
                if i > 0 {
                    self.list_state.select(Some(i - 1));
                    self.preview_dirty = true;
                }
            }
            KeyCode::Down => {
                let i = self.list_state.selected().unwrap_or(0);
                if i + 1 < self.filtered_indices.len() {
                    self.list_state.select(Some(i + 1));
                    self.preview_dirty = true;
                }
            }
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
                if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                    self.handle_global(key);
                } else {
                    self.query.insert(self.cursor_pos, c);
                    self.cursor_pos += 1;
                    self.apply_query();
                }
            }
            KeyCode::Enter => {
                self.handle_enter();
            }
            KeyCode::PageDown => {
                self.preview_scroll = self.preview_scroll.saturating_add(10);
            }
            KeyCode::PageUp => {
                self.preview_scroll = self.preview_scroll.saturating_sub(10);
            }
            KeyCode::Esc => {
                self.handle_esc();
            }
            _ => {}
        }
    }

    fn handle_global(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::{KeyCode, KeyModifiers};

        if matches!(key.modifiers, KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('c') | KeyCode::Char('C') => {
                    self.should_quit = true;
                }
                KeyCode::Char('a') | KeyCode::Char('A') => {
                    self.show_dotfiles = !self.show_dotfiles;
                    self.invalidate_find_cache();
                    self.refresh_items();
                }
                KeyCode::Char('w') | KeyCode::Char('W') => {
                    self.invalidate_find_cache();
                    self.toggle_winhidden();
                }
                KeyCode::Char('h') | KeyCode::Char('H') => {
                    if let Some(home) = dirs::home_dir() {
                        self.current_dir = home;
                        self.preview_text = Text::default();
                        self.invalidate_find_cache();
                        self.refresh_items();
                    }
                }
                KeyCode::Char('o') | KeyCode::Char('O') => {
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
                KeyCode::Char('g') | KeyCode::Char('G') => {
                    self.switch_mode();
                }
                _ => {}
            }
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
        cmd.args(["--vimgrep", "--smart-case", "--max-depth", "5"]);

        for d in crate::config::EXCLUDE_DIRS {
            cmd.args(["--glob", &format!("!{}", d)]);
        }
        for p in crate::config::EXCLUDE_PATH_GLOBS {
            cmd.args(["--glob", &format!("!{}", p)]);
        }
        if !self.show_winhidden {
            for d in crate::config::EXCLUDE_WIN_DIRS {
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
        let home = dirs::home_dir().unwrap_or_default();
        let home_str = home.to_string_lossy().replace('\\', "/");
        let mut zoxide_items: Vec<DirEntryItem> = Vec::new();
        let limit = 5;

        for zpath in &self.zoxide_cache {
            let exists = walker_items.iter().any(|w| w.full_path == *zpath);
            let is_current = *zpath == self.current_dir;
            if exists || is_current {
                continue;
            }
            let full_str = zpath.to_string_lossy().replace('\\', "/");
            let display = if full_str.starts_with(&home_str) {
                format!("~{}", &full_str[home_str.len()..])
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
