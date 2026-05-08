use std::io::stdout;
use std::path::PathBuf;
use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind, MouseEventKind};
use crossterm::execute;
use ratatui::text::Text;

use super::app::{App, ExitAction, Mode};

pub fn run(initial_query: Option<String>) -> anyhow::Result<Option<PathBuf>> {
    let mut terminal = ratatui::init();
    let mut app = App::new(initial_query)?;

    execute!(stdout(), crossterm::event::EnableMouseCapture)?;

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
            match event::read()? {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Press {
                        app.handle_key(key);
                    }
                }
                Event::Mouse(mouse) => {
                    if app.preview_text.lines.is_empty() {
                        continue;
                    }
                    match mouse.kind {
                        MouseEventKind::ScrollDown => {
                            app.preview_scroll = app.preview_scroll.saturating_add(3);
                        }
                        MouseEventKind::ScrollUp => {
                            app.preview_scroll = app.preview_scroll.saturating_sub(3);
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        } else if app.grep_pending && app.mode == Mode::Grep {
            app.run_grep_search();
        } else if app.find_pending {
            app.run_find_search();
        }

        if app.preview_dirty {
            app.preview_scroll = 0;
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

    execute!(stdout(), crossterm::event::DisableMouseCapture)?;
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
