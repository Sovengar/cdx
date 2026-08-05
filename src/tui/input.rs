use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::app::{App, Focus, Popup};

pub fn handle_key(app: &mut App, key: KeyEvent) {
    if app.popup.is_some() {
        handle_popup_key(app, key);
        return;
    }

    if app.handle_config_key(&key) {
        return;
    }

    match (app.focus, key.code) {
        (Focus::List, KeyCode::Up) => app.list_nav_up(),
        (Focus::List, KeyCode::Down) => app.list_nav_down(),
        (Focus::List, KeyCode::PageUp) => app.preview.scroll = app.preview.scroll.saturating_sub(10),
        (Focus::List, KeyCode::PageDown) => app.preview.scroll = app.preview.scroll.saturating_add(10),
        (Focus::List, KeyCode::Enter) => app.handle_enter(),
        (Focus::List, _) => handle_list_key(app, key),
        (Focus::Preview, _) => handle_preview_key(app, key),
    }
}

fn handle_popup_key(app: &mut App, key: KeyEvent) {
    match app.popup {
        Some(Popup::ToolSelector) => {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if app.popup_index > 0 {
                        app.popup_index -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    let tool_count = crate::config::get().tool_selector.len();
                    if tool_count > 0 && app.popup_index < tool_count - 1 {
                        app.popup_index += 1;
                    }
                }
                KeyCode::Enter => {
                    app.execute_tool_from_popup();
                }
                KeyCode::Esc | KeyCode::Char('q') => {
                    app.popup = None;
                }
                _ => {}
            }
        }
        None => {}
    }
}

fn handle_preview_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Up => {
            if app.preview.selection > 0 {
                app.preview.selection -= 1;
            }
        }
        KeyCode::Down => {
            if app.preview.selection + 1 < app.preview.entries.len() {
                app.preview.selection += 1;
            }
        }
        KeyCode::Enter if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if let Some(idx) = app.list_state.selected() {
                if let Some(&item_idx) = app.filtered_indices.get(idx) {
                    if let Some(item) = app.items.get(item_idx) {
                        if item.is_dir {
                            app.popup = Some(Popup::ToolSelector);
                            app.popup_index = 0;
                        }
                    }
                }
            }
        }
        KeyCode::Enter => {
            if let Some(entry) = app.preview.entries.get(app.preview.selection) {
                if entry.is_dir && entry.full_path.is_dir() {
                    app.current_dir = entry.full_path.clone();
                    app.query.clear();
                    app.cursor_pos = 0;
                    app.reset_preview();
                    app.focus = Focus::List;
                    app.invalidate_find_cache();
                    app.refresh_items();
                    app.preview.dirty = true;
                }
            }
        }
        KeyCode::Left | KeyCode::Esc => {
            app.focus = Focus::List;
        }
        KeyCode::Tab => {
            app.focus = Focus::List;
            app.switch_mode();
        }
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                app.handle_config_key(&key);
            } else {
                app.query.insert(app.cursor_pos, c);
                app.cursor_pos += 1;
                app.apply_query();
            }
        }
        _ => {}
    }
}

fn handle_list_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Left => {
            if app.cursor_pos > 0 {
                app.cursor_pos -= 1;
            }
        }
        KeyCode::Right => {
            let has_preview = !app.preview.text.lines.is_empty()
                || !app.preview.contents.lines.is_empty();
            if has_preview {
                app.focus = Focus::Preview;
            } else if app.cursor_pos < app.query.len() {
                app.cursor_pos += 1;
            }
        }
        KeyCode::Home => {
            app.cursor_pos = 0;
        }
        KeyCode::End => {
            app.cursor_pos = app.query.len();
        }
        KeyCode::Backspace => {
            if app.cursor_pos > 0 {
                app.query.remove(app.cursor_pos - 1);
                app.cursor_pos -= 1;
                app.apply_query();
            }
        }
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                app.handle_config_key(&key);
            } else {
                app.query.insert(app.cursor_pos, c);
                app.cursor_pos += 1;
                app.apply_query();
            }
        }
        KeyCode::Esc => {
            app.handle_esc();
        }
        _ => {}
    }
}
