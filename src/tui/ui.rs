use std::path::Path;

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph};
use ratatui::Frame;

use super::app::{App, Mode};
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
            .border_style(Style::default().fg(Color::Cyan))
            .title(" cdx ")
            .title_alignment(Alignment::Center);

        frame.render_widget(block.clone(), popup);
        let inner = block.inner(popup);

        layout_inner(inner)
    } else {
        layout_inner(term)
    };

    render_status(frame, outer[0], app);
    render_input(frame, outer[1], app);
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
        Constraint::Length(3),
    ])
    .split(area);

    let cols = Layout::horizontal([
        Constraint::Percentage(60),
        Constraint::Percentage(40),
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

    frame.render_stateful_widget(list, area, &mut app.list_state);
}

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn render_header(frame: &mut Frame, area: Rect, app: &mut App) {
    let label = match app.mode {
        Mode::Find => "Enter (cd) | Esc (..) | Ctrl+G (Search) | Ctrl+O (yazi) | Ctrl+A (.) | Ctrl+W (h) | Ctrl+H (~) | Ctrl+C (quit)",
        Mode::Search => "Enter (open) | Esc (..) | Ctrl+G (Grep) | Ctrl+H (~) | Ctrl+A (.) | Ctrl+W (h) | Ctrl+C (quit)",
        Mode::Grep => "Enter (cd parent) | Esc (..) | Ctrl+G (Find) | Ctrl+H (~) | Ctrl+A (.) | Ctrl+W (h) | Ctrl+C (quit)",
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(header_fg()))
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
    frame.render_widget(
        Paragraph::new(label).style(Style::default().fg(header_fg())),
        inner,
    );
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
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(header_fg()))
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
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(header_fg()))
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
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(header_fg()))
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

fn render_preview(frame: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::default()
        .borders(Borders::LEFT)
        .border_style(Style::default().fg(Color::DarkGray));

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

    let paragraph = Paragraph::new(app.preview_text.clone())
        .scroll((app.preview_scroll as u16, 0));

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
