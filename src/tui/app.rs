use std::path::PathBuf;
use std::time::{Duration, Instant};

use nucleo_matcher::{Matcher, Config};
use ratatui::widgets::ListState;

use crate::config;
use crate::walker::DirEntryItem;
use crate::zoxide;

use super::grep;
use super::navigation;
use super::preview_state::PreviewState;
use super::score;
use super::search::SearchState;

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
    pub preview: PreviewState,

    pub matcher: Matcher,
    pub scratch: Vec<char>,

    pub grep_pending: bool,
    pub grep_results: Vec<grep::GrepMatch>,
    pub grep_search_root: PathBuf,
    pub grep_debounce_ms: u64,

    pub search: SearchState,

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

        let mut search = SearchState::new(cfg.find_debounce_ms);
        if !query.is_empty() {
            search.find_deadline = Some(
                Instant::now() + Duration::from_millis(cfg.find_debounce_ms),
            );
        }

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
            preview: PreviewState::new(),

            matcher: Matcher::new(Config::DEFAULT.match_paths()),
            scratch: Vec::new(),

            grep_pending: false,
            grep_results: Vec::new(),
            grep_search_root: dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")),
            grep_debounce_ms: cfg.grep_debounce_ms,

            search,

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
            zoxide::merge::merge_with_dirs(
                &self.zoxide_cache,
                walker_items,
                &self.current_dir,
                config::get().zoxide_limit,
            )
        } else {
            walker_items
        };
        self.filtered_indices = (0..self.items.len()).collect();
        if !self.filtered_indices.is_empty() {
            self.list_state.select(Some(0));
            self.preview.dirty = true;
        }
    }

    pub(crate) fn invalidate_find_cache(&mut self) {
        self.search.invalidate();
    }

    pub fn apply_query(&mut self) {
        if self.mode == Mode::Grep {
            self.search.cancel();
            self.search.find_deadline = None;
            if !self.query.is_empty() {
                self.grep_pending = true;
            }
            return;
        }

        self.search.generation = self.search.generation.wrapping_add(1);

        if self.query.is_empty() {
            self.search.cancel();
            self.search.find_deadline = None;
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

        self.search.set_deadline(&self.query);
    }

    pub fn start_find_search(&mut self) {
        if !self.search.is_due() {
            return;
        }
        self.search.find_deadline = None;

        if self.query.is_empty() {
            return;
        }

        if self.mode == Mode::Find {
            if self.search.find_cache_root != self.current_dir || !self.search.find_cache_loaded {
                self.search.spawn_search(
                    self.current_dir.clone(),
                    self.show_dotfiles,
                    self.show_winhidden,
                );
            } else {
                self.filter_find_cache();
            }
            return;
        }

        // Search mode: score current items
        let scored = score::score_items(&self.query, &self.items, &mut self.matcher, &mut self.scratch);
        self.filtered_indices = scored.into_iter().map(|(i, _)| i).collect();

        if !self.filtered_indices.is_empty() {
            self.list_state.select(Some(0));
        }
    }

    fn filter_find_cache(&mut self) {
        let mut scored = score::score_items(&self.query, &self.search.find_cache, &mut self.matcher, &mut self.scratch);
        scored.truncate(200);

        let top_indices: Vec<usize> = scored.into_iter().map(|(i, _)| i).collect();
        self.items = top_indices
            .iter()
            .map(|&i| self.search.find_cache[i].clone())
            .collect();
        self.filtered_indices = (0..self.items.len()).collect();

        if !self.filtered_indices.is_empty() {
            self.list_state.select(Some(0));
        }
    }

    pub fn receive_search_results(&mut self) {
        if self.search.receive_results(&self.current_dir, &self.query) {
            self.filter_find_cache();
        }
    }

    pub fn search_poll_timeout(&self) -> Duration {
        self.search.poll_timeout()
    }

    pub fn search_due(&self) -> bool {
        self.search.is_due()
    }

    pub(crate) fn reset_preview(&mut self) {
        self.preview.reset();
    }

    pub fn handle_enter(&mut self) {
        let selected = self.list_state.selected();
        let idx = match selected.and_then(|s| self.filtered_indices.get(s)) {
            Some(&i) => i,
            None => return,
        };
        let item = match self.items.get(idx) {
            Some(item) => item.clone(),
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
                    self.preview.dirty = true;
                }
            }
            Mode::Search | Mode::Grep => {
                navigation::navigate_to_parent(self, &item);
            }
        }
    }

    pub fn handle_esc(&mut self) {
        navigation::handle_esc(self);
    }

    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        super::input::handle_key(self, key);
    }

    pub(crate) fn list_nav_up(&mut self) {
        let i = self.list_state.selected().unwrap_or(0);
        if i > 0 {
            self.list_state.select(Some(i - 1));
            self.preview.dirty = true;
        }
    }

    pub(crate) fn list_nav_down(&mut self) {
        let i = self.list_state.selected().unwrap_or(0);
        if i + 1 < self.filtered_indices.len() {
            self.list_state.select(Some(i + 1));
            self.preview.dirty = true;
        }
    }

    pub(crate) fn execute_tool_from_popup(&mut self) {
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

    pub(crate) fn get_selected_dir(&self) -> Option<PathBuf> {
        let idx = self.list_state.selected()?;
        let item_idx = self.filtered_indices.get(idx)?;
        let item = self.items.get(*item_idx)?;
        if item.is_dir {
            Some(item.full_path.clone())
        } else {
            None
        }
    }

    pub(crate) fn handle_config_key(&mut self, key: &crossterm::event::KeyEvent) -> bool {
        let action = crate::config::get().keys.match_action(key);
        match action {
            Some(action) => super::actions::execute_action(self, action),
            None => false,
        }
    }

    pub fn switch_mode(&mut self) {
        self.search.generation = self.search.generation.wrapping_add(1);
        self.search.cancel();
        self.search.find_deadline = None;
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
        self.preview.dirty = true;
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
        let (results, items) = grep::execute_search(
            &self.query,
            &root,
            self.show_winhidden,
            self.show_dotfiles,
        );

        self.grep_results = results;
        self.items = items;
        self.filtered_indices = (0..self.items.len()).collect();

        if !self.filtered_indices.is_empty() {
            self.list_state.select(Some(0));
        }
    }
}
