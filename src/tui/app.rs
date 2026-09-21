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
        self.invalidate_find_cache();
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
        self.preview.dirty = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;
    use std::fs;
    use std::path::Path;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    fn setup_test_dir(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("cdx-app-test-{}-{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn cleanup(root: &Path) {
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_app_new_default_state() {
        config::init();
        let app = App::new(None).unwrap();
        assert_eq!(app.mode, Mode::Find);
        assert_eq!(app.focus, Focus::List);
        assert!(app.popup.is_none());
        assert!(!app.should_quit);
        assert!(app.query.is_empty());
        assert_eq!(app.cursor_pos, 0);
    }

    #[test]
    fn test_app_new_with_initial_query() {
        config::init();
        let app = App::new(Some("hello".to_string())).unwrap();
        assert_eq!(app.query, "hello");
        assert_eq!(app.cursor_pos, 5);
        assert!(!app.zoxide_initial_visible, "query present → zoxide hidden");
    }

    // ── Mode switching ──────────────────────────────────────────

    #[test]
    fn test_mode_cycle_find_to_search() {
        config::init();
        let root = setup_test_dir("cycle1");
        fs::create_dir_all(root.join("sub")).unwrap();

        let mut app = App::new(None).unwrap();
        app.current_dir = root.clone();
        app.zoxide_initial_visible = false;
        app.refresh_items();
        assert_eq!(app.mode, Mode::Find);

        app.switch_mode();
        assert_eq!(app.mode, Mode::Search);
        cleanup(&root);
    }

    #[test]
    fn test_mode_cycle_search_to_grep() {
        config::init();
        let root = setup_test_dir("cycle2");
        let mut app = App::new(None).unwrap();
        app.current_dir = root.clone();
        app.mode = Mode::Search;

        app.switch_mode();
        assert_eq!(app.mode, Mode::Grep);
        cleanup(&root);
    }

    #[test]
    fn test_mode_cycle_grep_to_find() {
        config::init();
        let root = setup_test_dir("cycle3");
        let mut app = App::new(None).unwrap();
        app.current_dir = root.clone();
        app.mode = Mode::Grep;

        app.switch_mode();
        assert_eq!(app.mode, Mode::Find);
        cleanup(&root);
    }

    #[test]
    fn test_mode_cycle_full_round_trip() {
        config::init();
        let root = setup_test_dir("cycle-full");
        let mut app = App::new(None).unwrap();
        app.current_dir = root.clone();

        app.switch_mode(); // Find → Search
        app.switch_mode(); // Search → Grep
        app.switch_mode(); // Grep → Find
        assert_eq!(app.mode, Mode::Find);
        cleanup(&root);
    }

    #[test]
    fn test_grep_mode_disables_winhidden() {
        config::init();
        let root = setup_test_dir("grep-winhidden");
        let mut app = App::new(None).unwrap();
        app.current_dir = root.clone();
        app.show_winhidden = true;

        app.switch_mode(); // Find → Search
        app.switch_mode(); // Search → Grep
        assert!(!app.show_winhidden, "winhidden should be auto-disabled in grep");
        cleanup(&root);
    }

    #[test]
    fn test_switch_mode_cancels_search() {
        config::init();
        let root = setup_test_dir("switch-cancel");
        let mut app = App::new(None).unwrap();
        app.current_dir = root.clone();

        let cancel = Arc::new(AtomicBool::new(false));
        app.search.cancel = Some(Arc::clone(&cancel));

        app.switch_mode();
        assert!(cancel.load(std::sync::atomic::Ordering::Relaxed), "search should be cancelled");
        cleanup(&root);
    }

    #[test]
    fn test_switch_mode_invalidates_cache() {
        config::init();
        let root = setup_test_dir("switch-invalidate");
        let mut app = App::new(None).unwrap();
        app.current_dir = root.clone();
        // Manually populate cache to test invalidation
        app.search.find_cache = vec![DirEntryItem {
            display: "cached".into(),
            rel_path: "cached".into(),
            full_path: root.join("cached"),
            is_zoxide: false,
            is_dir: true,
        }];
        app.search.find_cache_root = root.clone();
        app.search.find_cache_loaded = true;
        let gen_before = app.search.generation;

        app.switch_mode();
        assert!(!app.search.find_cache_loaded, "cache should be invalidated");
        assert!(app.search.find_cache.is_empty(), "cache entries should be cleared");
        assert!(app.search.generation > gen_before, "generation should increment");
        cleanup(&root);
    }

    // ── Enter behavior ──────────────────────────────────────────

    #[test]
    fn test_enter_find_mode_navigates_to_dir() {
        config::init();
        let root = setup_test_dir("enter-find");
        fs::create_dir_all(root.join("target")).unwrap();

        let mut app = App::new(None).unwrap();
        app.current_dir = root.clone();
        // Manually set up items to avoid zoxide side effects
        let target = root.join("target");
        app.items = vec![DirEntryItem {
            display: "target".into(),
            rel_path: "target".into(),
            full_path: target.clone(),
            is_zoxide: false,
            is_dir: true,
        }];
        app.filtered_indices = vec![0];
        app.list_state.select(Some(0));

        app.handle_enter();
        assert_eq!(app.current_dir, target);
        assert!(app.query.is_empty(), "query should be cleared");
        cleanup(&root);
    }

    #[test]
    fn test_enter_find_mode_clears_query() {
        config::init();
        let root = setup_test_dir("enter-clear");
        fs::create_dir_all(root.join("sub")).unwrap();

        let mut app = App::new(None).unwrap();
        app.current_dir = root.clone();
        app.query = "sub".to_string();
        app.cursor_pos = 3;
        app.items = vec![DirEntryItem {
            display: "sub".into(),
            rel_path: "sub".into(),
            full_path: root.join("sub"),
            is_zoxide: false,
            is_dir: true,
        }];
        app.filtered_indices = vec![0];
        app.list_state.select(Some(0));

        app.handle_enter();
        assert!(app.query.is_empty());
        assert_eq!(app.cursor_pos, 0);
        cleanup(&root);
    }

    #[test]
    fn test_enter_no_selection_does_nothing() {
        config::init();
        let root = setup_test_dir("enter-noselect");
        let mut app = App::new(None).unwrap();
        app.current_dir = root.clone();
        let original_dir = app.current_dir.clone();

        app.handle_enter();
        assert_eq!(app.current_dir, original_dir);
        cleanup(&root);
    }

    // ── Esc behavior ────────────────────────────────────────────

    #[test]
    fn test_single_esc_navigates_to_parent() {
        config::init();
        let root = setup_test_dir("esc-parent");
        fs::create_dir_all(root.join("child")).unwrap();

        let mut app = App::new(None).unwrap();
        app.current_dir = root.join("child");

        app.handle_esc();
        assert_eq!(app.current_dir, root);
        cleanup(&root);
    }

    #[test]
    fn test_double_esc_navigates_to_home() {
        config::init();
        let root = setup_test_dir("esc-home");
        fs::create_dir_all(root.join("deep").join("nested")).unwrap();

        let mut app = App::new(None).unwrap();
        app.current_dir = root.join("deep").join("nested");

        app.handle_esc(); // first → goes to deep
        app.handle_esc(); // rapid → goes to home

        assert_eq!(app.current_dir, dirs::home_dir().unwrap());
        cleanup(&root);
    }

    #[test]
    fn test_esc_from_root_sets_should_quit() {
        config::init();
        let root = setup_test_dir("esc-root");
        let mut app = App::new(None).unwrap();
        app.current_dir = root.clone();

        // Set last_esc_time to something old so double-esc doesn't trigger
        app.last_esc_time = Some(std::time::Instant::now() - std::time::Duration::from_secs(1));

        // When parent is empty (root), should quit
        // But our test dir has a parent, so let's test the actual root "/"
        app.current_dir = PathBuf::from("/");
        app.last_esc_time = Some(std::time::Instant::now() - std::time::Duration::from_secs(1));
        app.handle_esc();
        // "/" parent is empty → should_quit = true
        assert!(app.should_quit);
        cleanup(&root);
    }

    // ── Tool selector ───────────────────────────────────────────

    #[test]
    fn test_get_selected_dir_returns_dir() {
        config::init();
        let root = setup_test_dir("get-dir");
        fs::create_dir_all(root.join("mydir")).unwrap();

        let mut app = App::new(None).unwrap();
        app.current_dir = root.clone();
        app.zoxide_initial_visible = false;
        app.refresh_items();
        app.list_state.select(Some(0));

        let dir = app.get_selected_dir();
        assert!(dir.is_some());
        assert_eq!(dir.unwrap(), root.join("mydir"));
        cleanup(&root);
    }

    #[test]
    fn test_get_selected_dir_no_selection() {
        config::init();
        let app = App::new(None).unwrap();
        assert!(app.get_selected_dir().is_none());
    }

    #[test]
    fn test_execute_tool_from_popup() {
        config::init();
        let root = setup_test_dir("tool-popup");
        fs::create_dir_all(root.join("mydir")).unwrap();

        let mut app = App::new(None).unwrap();
        app.current_dir = root.clone();
        app.zoxide_initial_visible = false;
        app.refresh_items();
        app.list_state.select(Some(0));
        app.popup = Some(Popup::ToolSelector);
        app.popup_index = 0; // yazi

        app.execute_tool_from_popup();
        assert!(app.popup.is_none(), "popup should close");
        assert!(app.spawn_pending.is_some(), "should have spawn pending");
        let (cmd, path) = app.spawn_pending.unwrap();
        assert_eq!(cmd, "yazi");
        assert_eq!(path, root.join("mydir"));
        cleanup(&root);
    }

    #[test]
    fn test_list_nav_up_down() {
        config::init();
        let root = setup_test_dir("nav-updown");
        fs::create_dir_all(root.join("a")).unwrap();
        fs::create_dir_all(root.join("b")).unwrap();

        let mut app = App::new(None).unwrap();
        app.current_dir = root.clone();
        app.zoxide_initial_visible = false;
        app.refresh_items();
        app.list_state.select(Some(1));

        app.list_nav_up();
        assert_eq!(app.list_state.selected(), Some(0));

        app.list_nav_down();
        assert_eq!(app.list_state.selected(), Some(1));

        app.list_nav_down(); // already at last
        assert_eq!(app.list_state.selected(), Some(1));

        app.list_state.select(Some(0));
        app.list_nav_up(); // already at first
        assert_eq!(app.list_state.selected(), Some(0));
        cleanup(&root);
    }
}
