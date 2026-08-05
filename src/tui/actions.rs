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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;
    use crate::tui::app::Mode;
    use std::fs;
    use std::path::Path;

    fn setup_test_dir(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("cdx-action-test-{}-{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn cleanup(root: &Path) {
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_quit_action() {
        config::init();
        let mut app = App::new(None).unwrap();
        assert!(!app.should_quit);
        let result = execute_action(&mut app, "quit");
        assert!(result);
        assert!(app.should_quit);
    }

    #[test]
    fn test_unknown_action_returns_false() {
        config::init();
        let mut app = App::new(None).unwrap();
        let result = execute_action(&mut app, "nonexistent");
        assert!(!result);
    }

    #[test]
    fn test_toggle_dotfiles() {
        config::init();
        let root = setup_test_dir("toggle-dot");
        let mut app = App::new(None).unwrap();
        app.current_dir = root.clone();
        let was = app.show_dotfiles;

        execute_action(&mut app, "toggle_dotfiles");
        assert_ne!(app.show_dotfiles, was);

        execute_action(&mut app, "toggle_dotfiles");
        assert_eq!(app.show_dotfiles, was);
        cleanup(&root);
    }

    #[test]
    fn test_toggle_winhidden() {
        config::init();
        let root = setup_test_dir("toggle-win");
        let mut app = App::new(None).unwrap();
        app.current_dir = root.clone();
        let was = app.show_winhidden;

        execute_action(&mut app, "toggle_winhidden");
        assert_ne!(app.show_winhidden, was);
        cleanup(&root);
    }

    #[test]
    fn test_open_settings() {
        config::init();
        let mut app = App::new(None).unwrap();
        let result = execute_action(&mut app, "open_settings");
        assert!(result);
        assert!(app.spawn_pending.is_some());
        let (cmd, path) = app.spawn_pending.unwrap();
        assert_eq!(cmd, "nvim");
        assert!(path.to_string_lossy().contains("config.toml"));
    }

    #[test]
    fn test_switch_mode_action() {
        config::init();
        let root = setup_test_dir("action-mode");
        let mut app = App::new(None).unwrap();
        app.current_dir = root.clone();
        assert_eq!(app.mode, Mode::Find);

        execute_action(&mut app, "switch_mode");
        assert_eq!(app.mode, Mode::Search);
        cleanup(&root);
    }

    #[test]
    fn test_open_explorer_with_dir_selected() {
        config::init();
        let root = setup_test_dir("explorer");
        fs::create_dir_all(root.join("target")).unwrap();

        let mut app = App::new(None).unwrap();
        app.current_dir = root.clone();
        app.refresh_items();
        app.list_state.select(Some(0));

        execute_action(&mut app, "open_explorer");
        assert_eq!(app.popup, Some(Popup::ToolSelector));
        assert_eq!(app.popup_index, 0);
        cleanup(&root);
    }

    #[test]
    fn test_open_explorer_no_selection() {
        config::init();
        let mut app = App::new(None).unwrap();
        execute_action(&mut app, "open_explorer");
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_go_home() {
        config::init();
        let root = setup_test_dir("go-home");
        fs::create_dir_all(root.join("deep")).unwrap();

        let mut app = App::new(None).unwrap();
        app.current_dir = root.join("deep");
        app.query = "test".to_string();

        execute_action(&mut app, "go_home");
        assert_eq!(app.current_dir, dirs::home_dir().unwrap());
        assert!(app.query.is_empty());
        cleanup(&root);
    }
}
