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
