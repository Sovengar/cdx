use std::path::Path;

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph};
use ratatui::Frame;

use super::app::{App, Focus, Mode};
use crate::walker::DirEntryItem;

const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

fn header_fg() -> Color {
    Color::Rgb(100, 200, 255)
}

fn path_fg() -> Color {
    Color::Rgb(150, 230, 255)
}

pub fn render(frame: &mut Frame, app: &mut App) {
    app.tick = app.tick.wrapping_add(1);
    let term = frame.area();

    let (outer, main) = if should_use_popup(term) {
        let w = std::cmp::min(term.width * app.popup_width_pct / 100, 120);
        let h = std::cmp::min(term.height * app.popup_height_pct / 100, 40);
        let x = (term.width - w) / 2;
        let y = (term.height - h) / 2;
        let popup = Rect::new(x, y, w, h);

        frame.render_widget(Clear, popup);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .title(Line::from(Span::styled(" CDX ", Style::default().add_modifier(Modifier::BOLD))))
            .title_alignment(Alignment::Center);

        frame.render_widget(block.clone(), popup);
        let inner = block.inner(popup);

        layout_inner(inner)
    } else {
        layout_inner(term)
    };

    render_status(frame, outer[0], app);
    let input_cols = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(outer[1]);
    render_input(frame, input_cols[0], app);
    render_info(frame, input_cols[1], app);
    render_list(frame, main[0], app);
    render_preview(frame, main[1], app);
    render_header(frame, outer[3], app);
}

fn should_use_popup(term: Rect) -> bool {
    term.width >= 80 && term.height >= 24
}

fn layout_inner(area: Rect) -> ([Rect; 4], [Rect; 2]) {
    let rows = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(4),
    ])
    .split(area);

    let cols = Layout::horizontal([
        Constraint::Percentage(50),
        Constraint::Percentage(50),
    ])
    .split(rows[2]);

    let outer: [Rect; 4] = [rows[0], rows[1], rows[2], rows[3]];
    let main: [Rect; 2] = [cols[0], cols[1]];
    (outer, main)
}

fn icon_for(item: &DirEntryItem) -> &'static str {
    if item.is_zoxide {
        return "★";
    }
    if item.is_dir {
        return "";
    }
    match Path::new(&item.display)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
    {
        "rs" => "",
        "toml" | "yml" | "yaml" | "json" => "",
        "md" | "txt" => "",
        "ps1" | "sh" | "bat" | "cmd" => "",
        "exe" | "dll" => "",
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "ico" => "",
        "gitignore" | "gitattributes" | "gitmodules" => "",
        _ => "",
    }
}

fn style_for(item: &DirEntryItem) -> Style {
    if item.is_zoxide {
        return Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
    }
    if item.is_dir {
        return Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
    }
    match Path::new(&item.display)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
    {
        "rs" => Style::default().fg(Color::LightBlue),
        "toml" | "yml" | "yaml" | "json" => Style::default().fg(Color::LightRed),
        "md" | "txt" => Style::default().fg(Color::LightYellow),
        "ps1" | "sh" | "bat" => Style::default().fg(Color::LightGreen),
        "exe" | "dll" => Style::default().fg(Color::LightCyan),
        "png" | "jpg" | "jpeg" | "gif" | "svg" => Style::default().fg(Color::LightMagenta),
        _ => Style::default().fg(Color::White),
    }
}

fn render_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let focus_fg = if app.focus == Focus::List {
        Color::Yellow
    } else {
        header_fg()
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(focus_fg).add_modifier(Modifier::BOLD))
        .title(" Files ")
        .title(
            Line::from(Span::styled(
                format!(" {}/{} ", app.filtered_indices.len(), app.items.len()),
                Style::default().fg(if app.focus == Focus::List { Color::Rgb(180, 100, 255) } else { Color::Cyan }),
            ))
            .right_aligned(),
        );

    let inner = block.inner(area);

    let items: Vec<ListItem> = app
        .filtered_indices
        .iter()
        .filter_map(|&i| app.items.get(i))
        .map(|item| {
            let icon = icon_for(item);
            let style = style_for(item);
            let display = format!("{} {}", icon, item.display);
            ListItem::new(display).style(style)
        })
        .collect();

    let list = List::new(items)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("▶ ");

    if !app.preview_contents.lines.is_empty() {
        let chunks = Layout::vertical([
            Constraint::Fill(3),
            Constraint::Length(1),
            Constraint::Fill(2),
        ])
        .split(inner);

        frame.render_widget(block, area);
        frame.render_stateful_widget(list, chunks[0], &mut app.list_state);

        let sep = Line::from(Span::styled(
            "─".repeat(chunks[1].width as usize),
            Style::default().fg(Color::DarkGray),
        ));
        frame.render_widget(Paragraph::new(sep), chunks[1]);

        let eza_text = {
            let mut t = app.preview_contents.clone();
            for line in t.lines.iter_mut() {
                for span in line.spans.iter_mut() {
                    let mut s = "  ".to_string();
                    s.push_str(&span.content);
                    span.content = s.into();
                }
            }
            t
        };
        frame.render_widget(
            Paragraph::new(eza_text),
            chunks[2],
        );
    } else {
        frame.render_widget(block, area);
        frame.render_stateful_widget(list, inner, &mut app.list_state);
    }
}

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn render_header(frame: &mut Frame, area: Rect, app: &mut App) {
    let labels = match app.mode {
        Mode::Find => vec![
            "Enter (cd) | Esc (..) | Esc² (~) | Tab (Search) | Ctrl+Enter (explorer)",
            "Ctrl+A (.) | Ctrl+W (h) | Ctrl+E (settings) | Ctrl+C (quit)",
        ],
        Mode::Search => vec![
            "Enter (open) | Esc (..) | Esc² (~) | Tab (Grep)",
            "Ctrl+A (.) | Ctrl+W (h) | Ctrl+E (settings) | Ctrl+C (quit)",
        ],
        Mode::Grep => vec![
            "Enter (cd parent) | Esc (..) | Esc² (~) | Tab (Find)",
            "Ctrl+A (.) | Ctrl+W (h) | Ctrl+E (settings) | Ctrl+C (quit)",
        ],
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(header_fg()).add_modifier(Modifier::BOLD))
        .title(" Legend ")
        .title_bottom(
            Line::from(Span::styled(
                format!(" v{} ", VERSION),
                Style::default().fg(Color::Cyan),
            ))
            .right_aligned(),
        );

    let inner = block.inner(area);

    frame.render_widget(block, area);
    let lines: Vec<Line> = labels.iter().map(|l| {
        Line::from(Span::styled(
            format!("  {}", l),
            Style::default().fg(header_fg()).add_modifier(Modifier::BOLD),
        ))
    }).collect();
    frame.render_widget(Paragraph::new(lines), inner);
}

fn render_status(frame: &mut Frame, area: Rect, app: &mut App) {
    let path = match app.mode {
        Mode::Grep => display_path(&app.grep_search_root),
        _ => display_path(&app.current_dir),
    };
    let mode = match app.mode {
        Mode::Find => "Find",
        Mode::Search => "Search",
        Mode::Grep => "Grep",
    };
    let dot = if app.show_dotfiles { "✓" } else { "✗" };
    let win = if app.show_winhidden { "✓" } else { "✗" };

    let spinner = if app.find_pending || app.grep_pending {
        let idx = (app.tick as usize / 3) % SPINNER.len();
        SPINNER[idx]
    } else {
        ""
    };

    let cols = Layout::horizontal([Constraint::Fill(1), Constraint::Length(38)])
        .split(area);

    let path_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(header_fg()).add_modifier(Modifier::BOLD))
        .title(" Path ");

    let path_inner = path_block.inner(cols[0]);
    frame.render_widget(path_block, cols[0]);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!("{}  ", spinner), Style::default().fg(header_fg())),
            Span::styled(
                path,
                Style::default().fg(path_fg()).add_modifier(Modifier::BOLD),
            ),
        ])),
        path_inner,
    );

    let status_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(header_fg()).add_modifier(Modifier::BOLD))
        .title(" Search configuration ");

    let status_inner = status_block.inner(cols[1]);
    frame.render_widget(status_block, cols[1]);
    frame.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            format!(" {} | dotfiles: {} | WinHidden: {}", mode, dot, win),
            Style::default().fg(header_fg()),
        )])),
        status_inner,
    );
}

fn render_input(frame: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(header_fg()).add_modifier(Modifier::BOLD))
        .title(" \u{1F50D} ");

    let inner = block.inner(area);
    let prefix = "> ";
    let text = format!("{}{}", prefix, app.query);

    frame.render_widget(block, area);
    frame.render_widget(
        Paragraph::new(text).style(Style::default().fg(Color::LightGreen)),
        inner,
    );

    let cursor_x = inner.x + 2 + app.cursor_pos as u16;
    let cursor_y = inner.y;
    frame.set_cursor_position((cursor_x, cursor_y));
}

fn render_info(frame: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(header_fg()).add_modifier(Modifier::BOLD))
        .title(" Info ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    // Selected item info
    if let Some(idx) = app.list_state.selected() {
        if let Some(&item_idx) = app.filtered_indices.get(idx) {
            if let Some(item) = app.items.get(item_idx) {
                lines.push(Line::from(Span::styled(
                    format!(" {} {}", icon_for(item), item.display),
                    style_for(item),
                )));
                if let Ok(meta) = std::fs::metadata(&item.full_path) {
                    if meta.is_file() {
                        let size = meta.len();
                        let size_str = if size > 1024 * 1024 {
                            format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
                        } else if size > 1024 {
                            format!("{:.1} KB", size as f64 / 1024.0)
                        } else {
                            format!("{} B", size)
                        };
                        lines.push(Line::from(Span::styled(format!(" size: {}", size_str), Style::default().fg(Color::DarkGray))));
                    }
                    if let Ok(modified) = meta.modified() {
                        if let Ok(elapsed) = modified.elapsed() {
                            let age = if elapsed.as_secs() < 3600 {
                                format!("{}m ago", elapsed.as_secs() / 60)
                            } else if elapsed.as_secs() < 86400 {
                                format!("{}h ago", elapsed.as_secs() / 3600)
                            } else {
                                format!("{}d ago", elapsed.as_secs() / 86400)
                            };
                            lines.push(Line::from(Span::styled(format!(" modified: {}", age), Style::default().fg(Color::DarkGray))));
                        }
                    }
                }
            }
        }
    }

    // Git info
    let git_toplevel = std::process::Command::new("git")
        .args(["-C", &app.current_dir.to_string_lossy(), "rev-parse", "--show-toplevel"])
        .output()
        .ok();
    if let Some(out) = git_toplevel {
        if out.status.success() {
            let top = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let branch = std::process::Command::new("git")
                .args(["-C", &app.current_dir.to_string_lossy(), "rev-parse", "--abbrev-ref", "HEAD"])
                .output()
                .ok()
                .and_then(|o| if o.status.success() { Some(String::from_utf8_lossy(&o.stdout).trim().to_string()) } else { None });
            let status = std::process::Command::new("git")
                .args(["-C", &app.current_dir.to_string_lossy(), "status", "--porcelain"])
                .output()
                .ok();
            let dirty = status.map_or(false, |o| !o.stdout.is_empty());
            let display_path = display_path(&std::path::PathBuf::from(&top));
            lines.push(Line::from(Span::styled(" git:", Style::default().fg(header_fg()).add_modifier(Modifier::BOLD))));
            lines.push(Line::from(Span::styled(format!("  {}", display_path), Style::default().fg(Color::Cyan))));
            if let Some(b) = branch {
                lines.push(Line::from(Span::styled(
                    format!("  {} ({})", b, if dirty { "dirty" } else { "clean" }),
                    if dirty { Style::default().fg(Color::LightRed) } else { Style::default().fg(Color::LightGreen) },
                )));
            }
        }
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

fn render_preview(frame: &mut Frame, area: Rect, app: &mut App) {
    let focus_fg = if app.focus == Focus::Preview {
        Color::Yellow
    } else {
        header_fg()
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(focus_fg).add_modifier(Modifier::BOLD))
        .title(" Tree ");

    let inner = block.inner(area);

    if app.preview_text.lines.is_empty() {
        frame.render_widget(
            Paragraph::new(" (select an item) ")
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center),
            inner,
        );
        frame.render_widget(block, area);
        return;
    }

    let mut text = app.preview_text.clone();
    if app.focus == Focus::Preview && !app.preview_entries.is_empty() {
        if let Some(entry) = app.preview_entries.get(app.preview_selection) {
            // auto-scroll to keep selected entry visible
            let visible_lines = inner.height as u64;
            if (entry.line_index as u64) >= app.preview_scroll + visible_lines {
                app.preview_scroll = (entry.line_index as u64).saturating_sub(visible_lines) + 1;
            } else if (entry.line_index as u64) < app.preview_scroll {
                app.preview_scroll = entry.line_index as u64;
            }

            if entry.line_index < text.lines.len() {
                let line = &text.lines[entry.line_index];
                let mut spans: Vec<Span> = line.spans.iter().map(|s| {
                    Span::styled(s.content.clone(), s.style.add_modifier(Modifier::REVERSED))
                }).collect();
                if let Some(last) = spans.last_mut() {
                    last.content.to_mut().push_str(&" ".repeat(inner.width as usize));
                }
                text.lines[entry.line_index] = Line::from(spans);
            }
        }
    }

    let paragraph = Paragraph::new(text).scroll((app.preview_scroll as u16, 0));

    frame.render_widget(block, area);
    frame.render_widget(paragraph, inner);
}

fn display_path(path: &std::path::Path) -> String {
    let s = path.to_string_lossy().replace('\\', "/");
    if let Some(home) = dirs::home_dir() {
        let home_str = home.to_string_lossy().replace('\\', "/");
        if s.starts_with(&home_str) {
            return format!("~{}", &s[home_str.len()..]);
        }
    }
    s
}
