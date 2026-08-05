use std::io::stdout;
use std::path::PathBuf;
use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind, MouseEventKind};
use crossterm::execute;
use ratatui::text::Text;

use super::app::{App, ExitAction, Mode};

pub fn run(initial_query: Option<String>) -> anyhow::Result<Option<PathBuf>> {
    let mut terminal = ratatui::init();
    let mut app = App::new(initial_query.clone())?;

    execute!(stdout(), crossterm::event::EnableMouseCapture)?;

    app.refresh_items();

    while !app.should_quit {
        app.receive_search_results();
        terminal.draw(|f| super::ui::render(f, &mut app))?;

        let timeout = if app.mode == Mode::Grep && app.grep_pending {
            Duration::from_millis(app.grep_debounce_ms)
        } else {
            app.search_poll_timeout()
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
        } else if app.search_due() {
            app.start_find_search();
        }

        if app.preview_dirty {
            app.preview_scroll = 0;
            app.preview_entries.clear();
            app.preview_selection = 0;
            if let Some(idx) = app.list_state.selected() {
                if let Some(&item_idx) = app.filtered_indices.get(idx) {
                    if let Some(item) = app.items.get(item_idx) {
                        app.preview_text = crate::preview::generate(&app, item);
                        if item.is_dir {
                            let full_path = app.current_dir.join(&item.rel_path);
                            app.preview_contents = crate::preview::directory_contents(&full_path);
                            app.preview_entries = crate::preview::generate_entries(&app, item);
                        } else {
                            app.preview_contents = Text::default();
                        }
                    }
                }
            }
            app.preview_dirty = false;
        }

        // Handle inline tool spawning
        if let Some((ref command, ref path)) = app.spawn_pending {
            let command = command.clone();
            let path = path.clone();
            app.spawn_pending = None;

            execute!(stdout(), crossterm::event::DisableMouseCapture)?;
            ratatui::restore();

            let _ = std::process::Command::new(&command)
                .arg(&path)
                .status();

            terminal = ratatui::init();
            execute!(stdout(), crossterm::event::EnableMouseCapture)?;
            app.preview_dirty = true;
        }
    }

    execute!(stdout(), crossterm::event::DisableMouseCapture)?;
    ratatui::restore();

    if let ExitAction::OutputPath(path) = &app.exit_action {
        return Ok(Some(path.clone()));
    }

    Ok(Some(app.current_dir))
}
