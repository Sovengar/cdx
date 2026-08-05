use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use crate::walker::DirEntryItem;

struct SearchResult {
    generation: u64,
    root: PathBuf,
    entries: Vec<DirEntryItem>,
}

pub struct SearchState {
    pub find_debounce_ms: u64,
    pub find_cache: Vec<DirEntryItem>,
    pub find_cache_root: PathBuf,
    pub find_cache_loaded: bool,
    pub find_deadline: Option<Instant>,
    pub generation: u64,
    pub cancel: Option<Arc<AtomicBool>>,
    tx: mpsc::Sender<SearchResult>,
    rx: mpsc::Receiver<SearchResult>,
}

impl SearchState {
    pub fn new(debounce_ms: u64) -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            find_debounce_ms: debounce_ms,
            find_cache: Vec::new(),
            find_cache_root: PathBuf::new(),
            find_cache_loaded: false,
            find_deadline: None,
            generation: 0,
            cancel: None,
            tx,
            rx,
        }
    }

    pub fn invalidate(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.cancel();
        self.find_cache.clear();
        self.find_cache_root.clear();
        self.find_cache_loaded = false;
        self.find_deadline = None;
    }

    pub fn cancel(&mut self) {
        if let Some(cancel) = self.cancel.take() {
            cancel.store(true, Ordering::Relaxed);
        }
    }

    pub fn set_deadline(&mut self, query: &str) {
        self.cancel();
        if query.is_empty() {
            self.find_deadline = None;
        } else {
            self.find_deadline = Some(
                Instant::now() + Duration::from_millis(self.find_debounce_ms),
            );
        }
    }

    pub fn is_due(&self) -> bool {
        self.find_deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
    }

    pub fn poll_timeout(&self) -> Duration {
        let base = Duration::from_millis(50);
        self.find_deadline
            .map(|deadline| deadline.saturating_duration_since(Instant::now()).min(base))
            .unwrap_or(base)
    }

    pub fn spawn_search(
        &mut self,
        root: PathBuf,
        show_dotfiles: bool,
        show_winhidden: bool,
    ) {
        let generation = self.generation;
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = Arc::clone(&cancel);
        let sender = self.tx.clone();

        self.cancel = Some(cancel);
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

    pub fn receive_results(
        &mut self,
        current_dir: &PathBuf,
        query: &str,
    ) -> bool {
        while let Ok(result) = self.rx.try_recv() {
            if query.is_empty()
                || result.generation != self.generation
                || result.root != *current_dir
            {
                continue;
            }

            self.find_cache = result.entries;
            self.find_cache_root = result.root;
            self.find_cache_loaded = true;
            self.cancel = None;
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_search_state_new_defaults() {
        let state = SearchState::new(50);
        assert_eq!(state.find_debounce_ms, 50);
        assert!(state.find_cache.is_empty());
        assert!(!state.find_cache_loaded);
        assert!(state.find_deadline.is_none());
        assert_eq!(state.generation, 0);
        assert!(state.cancel.is_none());
    }

    #[test]
    fn test_debounce_no_deadline_not_due() {
        let state = SearchState::new(50);
        assert!(!state.is_due());
    }

    #[test]
    fn test_debounce_just_set_not_due() {
        let mut state = SearchState::new(50);
        state.set_deadline("query");
        assert!(!state.is_due());
    }

    #[test]
    fn test_debounce_after_wait_is_due() {
        let mut state = SearchState::new(50);
        state.set_deadline("query");
        thread::sleep(Duration::from_millis(60));
        assert!(state.is_due());
    }

    #[test]
    fn test_debounce_resets_on_new_query() {
        let mut state = SearchState::new(50);
        state.set_deadline("a");
        thread::sleep(Duration::from_millis(30));
        state.set_deadline("ab"); // resets
        assert!(!state.is_due()); // 30ms < 50ms from second set
    }

    #[test]
    fn test_debounce_empty_query_clears_deadline() {
        let mut state = SearchState::new(50);
        state.set_deadline("query");
        assert!(state.find_deadline.is_some());
        state.set_deadline("");
        assert!(state.find_deadline.is_none());
    }

    #[test]
    fn test_cancel_sets_flag() {
        let mut state = SearchState::new(50);
        let cancel = Arc::new(AtomicBool::new(false));
        state.cancel = Some(Arc::clone(&cancel));

        state.cancel();
        assert!(cancel.load(Ordering::Relaxed));
        assert!(state.cancel.is_none(), "cancel is taken after cancel()");
    }

    #[test]
    fn test_cancel_no_flag_is_noop() {
        let mut state = SearchState::new(50);
        state.cancel(); // should not panic
    }

    #[test]
    fn test_generation_increment() {
        let mut state = SearchState::new(50);
        let gen_before = state.generation;
        state.generation = state.generation.wrapping_add(1);
        assert_eq!(state.generation, gen_before + 1);
    }

    #[test]
    fn test_generation_wraps_around() {
        let mut state = SearchState::new(50);
        state.generation = u64::MAX;
        state.generation = state.generation.wrapping_add(1);
        assert_eq!(state.generation, 0);
    }

    #[test]
    fn test_invalidate_clears_cache_and_increments_generation() {
        let mut state = SearchState::new(50);
        state.find_cache = vec![DirEntryItem {
            display: "test".into(),
            rel_path: "test".into(),
            full_path: PathBuf::from("/test"),
            is_zoxide: false,
            is_dir: true,
        }];
        state.find_cache_root = PathBuf::from("/test");
        state.find_cache_loaded = true;
        state.find_deadline = Some(Instant::now());

        let gen_before = state.generation;
        state.invalidate();

        assert!(state.find_cache.is_empty());
        assert!(!state.find_cache_loaded);
        assert!(state.find_deadline.is_none());
        assert_eq!(state.generation, gen_before + 1);
    }

    #[test]
    fn test_poll_timeout_no_deadline() {
        let state = SearchState::new(50);
        assert_eq!(state.poll_timeout(), Duration::from_millis(50));
    }

    #[test]
    fn test_poll_timeout_with_deadline() {
        let mut state = SearchState::new(100);
        state.find_deadline = Some(Instant::now() + Duration::from_millis(30));
        let timeout = state.poll_timeout();
        assert!(timeout <= Duration::from_millis(30));
        assert!(timeout <= Duration::from_millis(50)); // capped at base
    }

    #[test]
    fn test_receive_results_wrong_generation_ignored() {
        let mut state = SearchState::new(50);
        let root = PathBuf::from("/test");

        // Send results with generation 0, but set state to generation 1
        state.tx.send(SearchResult {
            generation: 0,
            root: root.clone(),
            entries: vec![],
        }).unwrap();
        state.generation = 1;

        let received = state.receive_results(&root, "query");
        assert!(!received);
        assert!(state.find_cache.is_empty());
    }

    #[test]
    fn test_receive_results_wrong_root_ignored() {
        let mut state = SearchState::new(50);

        state.tx.send(SearchResult {
            generation: 0,
            root: PathBuf::from("/other"),
            entries: vec![],
        }).unwrap();

        let received = state.receive_results(&PathBuf::from("/test"), "query");
        assert!(!received);
    }

    #[test]
    fn test_receive_results_empty_query_ignored() {
        let mut state = SearchState::new(50);

        state.tx.send(SearchResult {
            generation: 0,
            root: PathBuf::from("/test"),
            entries: vec![],
        }).unwrap();

        let received = state.receive_results(&PathBuf::from("/test"), "");
        assert!(!received);
    }

    #[test]
    fn test_receive_results_valid_accepted() {
        let mut state = SearchState::new(50);
        let root = PathBuf::from("/test");

        state.tx.send(SearchResult {
            generation: 0,
            root: root.clone(),
            entries: vec![DirEntryItem {
                display: "found".into(),
                rel_path: "found".into(),
                full_path: PathBuf::from("/test/found"),
                is_zoxide: false,
                is_dir: true,
            }],
        }).unwrap();

        let received = state.receive_results(&root, "query");
        assert!(received);
        assert_eq!(state.find_cache.len(), 1);
        assert_eq!(state.find_cache[0].display, "found");
        assert!(state.find_cache_loaded);
    }
}
