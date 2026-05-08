use std::path::PathBuf;
use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind};
use ratatui::text::Text;

use super::app::{App, ExitAction, Mode};

pub fn run(initial_query: Option<String>) -> anyhow::Result<Option<PathBuf>> {
    let mut terminal = ratatui::init();
    let mut app = App::new(initial_query)?;

    app.refresh_items();

    while !app.should_quit {
        terminal.draw(|f| super::ui::render(f, &mut app))?;

        let timeout = if app.mode == Mode::Grep && app.grep_pending {
            Duration::from_millis(app.grep_debounce_ms)
        } else if app.find_pending {
            Duration::from_millis(app.find_debounce_ms)
        } else {
            Duration::from_millis(50)
        };

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    app.handle_key(key);
                }
            }
        } else if app.grep_pending && app.mode == Mode::Grep {
            app.run_grep_search();
        } else if app.find_pending {
            app.run_find_search();
        }

        if app.preview_dirty {
            if let Some(idx) = app.list_state.selected() {
                if let Some(&item_idx) = app.filtered_indices.get(idx) {
                    if let Some(item) = app.items.get(item_idx) {
                        app.preview_text = crate::preview::generate(&app, item);
                        if item.is_dir {
                            let full_path = app.current_dir.join(&item.rel_path);
                            app.preview_contents = crate::preview::directory_contents(&full_path);
                        } else {
                            app.preview_contents = Text::default();
                        }
                    }
                }
            }
            app.preview_dirty = false;
        }
    }

    ratatui::restore();

    if let ExitAction::SpawnYazi(path) = &app.exit_action {
        std::process::Command::new("yazi")
            .arg(path)
            .status()?;
        return Ok(Some(app.current_dir));
    }

    if let ExitAction::OutputPath(path) = &app.exit_action {
        return Ok(Some(path.clone()));
    }

    Ok(Some(app.current_dir))
}
