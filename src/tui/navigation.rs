use std::path::PathBuf;
use std::time::Duration;

use super::app::{App, Mode};
use crate::walker::DirEntryItem;

pub fn navigate_to(app: &mut App, directory: PathBuf) {
    app.current_dir = directory;
    if app.mode == Mode::Grep {
        app.grep_search_root = app.current_dir.clone();
    }

    app.reset_preview();
    app.invalidate_find_cache();
    app.items.clear();
    app.filtered_indices.clear();
    app.list_state.select(None);
    app.grep_results.clear();
    app.grep_pending = false;

    app.refresh_items();
    app.apply_query();
    app.preview.dirty = true;
}

pub fn navigate_to_parent(app: &mut App, item: &DirEntryItem) {
    if let Some(parent) = item.full_path.parent() {
        if parent.exists() {
            app.current_dir = parent.to_path_buf();
            app.query.clear();
            app.cursor_pos = 0;
            app.reset_preview();
            app.mode = Mode::Find;
            app.invalidate_find_cache();
            app.refresh_items();
            app.preview.dirty = true;
        }
    }
}

pub fn handle_esc(app: &mut App) {
    if let Some(last) = app.last_esc_time {
        if last.elapsed() < Duration::from_millis(300) {
            app.last_esc_time = None;
            if let Some(home) = dirs::home_dir() {
                navigate_to(app, home);
            }
            return;
        }
    }
    app.last_esc_time = Some(std::time::Instant::now());
    if let Some(parent) = app.current_dir.parent() {
        if parent.as_os_str().is_empty() {
            app.should_quit = true;
        } else {
            let parent = parent.to_path_buf();
            if parent.exists() {
                navigate_to(app, parent);
            }
        }
    } else {
        app.should_quit = true;
    }
}
