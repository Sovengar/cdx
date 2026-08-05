use std::path::PathBuf;

use super::app::{App, Popup};

pub fn execute_action(app: &mut App, action: &str) -> bool {
    match action {
        "quit" => {
            app.should_quit = true;
            true
        }
        "toggle_dotfiles" => {
            app.show_dotfiles = !app.show_dotfiles;
            app.invalidate_find_cache();
            app.refresh_items();
            app.apply_query();
            true
        }
        "toggle_winhidden" => {
            app.invalidate_find_cache();
            app.toggle_winhidden();
            app.apply_query();
            true
        }
        "open_settings" => {
            let cfg_path = dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".config")
                .join("cdx")
                .join("config.toml");
            app.spawn_pending = Some(("nvim".to_string(), cfg_path));
            true
        }
        "switch_mode" => {
            app.switch_mode();
            true
        }
        "open_explorer" => {
            if app.get_selected_dir().is_some() {
                app.popup = Some(Popup::ToolSelector);
                app.popup_index = 0;
            }
            true
        }
        "go_home" => {
            if let Some(home) = dirs::home_dir() {
                app.current_dir = home;
                app.query.clear();
                app.cursor_pos = 0;
                app.reset_preview();
                app.invalidate_find_cache();
                app.refresh_items();
                app.preview.dirty = true;
            }
            true
        }
        _ => false,
    }
}
