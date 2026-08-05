pub mod commands;
pub mod git;
pub mod tree;

use std::path::{Path, PathBuf};

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};

use crate::tui::app::{App, Mode};
use crate::walker::DirEntryItem;

#[derive(Debug, Clone)]
pub struct PreviewEntry {
    #[allow(dead_code)]
    pub name: String,
    pub full_path: PathBuf,
    pub is_dir: bool,
    pub line_index: usize,
}

pub fn generate(app: &App, item: &DirEntryItem) -> Text<'static> {
    if app.mode == Mode::Grep {
        return commands::grep_preview(&item.full_path, &app.query);
    }

    let full_path = app.current_dir.join(&item.rel_path);

    if item.is_dir {
        preview_directory(&full_path, app.show_dotfiles, app.show_winhidden)
    } else {
        commands::preview_file(&full_path)
    }
}

fn header_line(label: &str) -> Line<'static> {
    Line::from(Span::styled(
        label.to_string(),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    ))
}

pub fn generate_entries(app: &App, item: &DirEntryItem) -> Vec<PreviewEntry> {
    if app.mode != Mode::Grep && item.is_dir {
        let full_path = app.current_dir.join(&item.rel_path);
        let mut entries = Vec::new();
        let mut line_counter = 0;
        let mut ctx = tree::TreeContext {
            show_dotfiles: app.show_dotfiles,
            show_winhidden: app.show_winhidden,
            out_entries: &mut entries,
            line_counter: &mut line_counter,
        };
        tree::build_tree_lines(&full_path, 2, 0, "", &mut ctx);
        entries
    } else {
        Vec::new()
    }
}

fn preview_directory(path: &Path, show_dotfiles: bool, show_winhidden: bool) -> Text<'static> {
    let mut lines: Vec<Line<'static>> = Vec::new();

    let mut entries = Vec::new();
    let mut line_counter = 0;
    let mut ctx = tree::TreeContext {
        show_dotfiles,
        show_winhidden,
        out_entries: &mut entries,
        line_counter: &mut line_counter,
    };
    let tree = tree::build_tree_lines(path, 2, 0, "", &mut ctx);
    if tree.is_empty() {
        lines.push(Line::from(Span::styled(
            " (empty)".to_string(),
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        lines.extend(tree);
    }

    lines.push(Line::from(""));
    lines.push(header_line("=== GIT STATUS ==="));
    lines.extend(git::git_status_lines(path));

    Text::from(lines)
}

