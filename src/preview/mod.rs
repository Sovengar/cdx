pub mod commands;
pub mod git;

use std::fs;
use std::path::{Path, PathBuf};

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};

use crate::tui::app::{App, Mode};
use crate::walker::DirEntryItem;
use crate::ui::icons::icon_for_name;

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
        build_tree_with_entries(&full_path, 2, 0, "", app.show_dotfiles, app.show_winhidden, &mut entries, &mut line_counter);
        entries
    } else {
        Vec::new()
    }
}

fn build_tree_with_entries(
    path: &Path,
    max_depth: usize,
    current_depth: usize,
    prefix: &str,
    show_dotfiles: bool,
    show_winhidden: bool,
    out_entries: &mut Vec<PreviewEntry>,
    line_counter: &mut usize,
) -> Vec<Line<'static>> {
    let mut items: Vec<(String, bool)> = Vec::new();

    if let Ok(dir_entries) = fs::read_dir(path) {
        for entry in dir_entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();

            if crate::config::get().exclude_dirs.contains(&name.as_str()) {
                continue;
            }
            if !show_dotfiles && name.starts_with('.') {
                continue;
            }
            if !show_winhidden {
                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    if let Ok(meta) = entry.metadata() {
                        if (meta.file_attributes() & 0x2) != 0 {
                            continue;
                        }
                    }
                }
            }

            let is_dir = entry.file_type().map_or(false, |t| t.is_dir());
            items.push((name, is_dir));
        }
    }

    items.sort_by(|a, b| {
        if a.1 != b.1 {
            b.1.cmp(&a.1)
        } else {
            a.0.cmp(&b.0)
        }
    });

    let total_files = items.iter().filter(|(_, d)| !*d).count();
    if total_files > 3 {
        let mut trimmed: Vec<(String, bool)> = Vec::new();
        let mut file_count = 0;
        for (name, is_dir) in &items {
            if !*is_dir {
                file_count += 1;
                if file_count > 3 {
                    continue;
                }
            }
            trimmed.push((name.clone(), *is_dir));
        }
        trimmed.push((format!("⋯ ({} more)", total_files - 3), false));
        items = trimmed;
    }

    let mut lines: Vec<Line<'static>> = Vec::new();
    let count = items.len();

    for (i, (name, is_dir)) in items.iter().enumerate() {
        let is_last = i == count - 1;
        let branch = if current_depth == 0 && !*is_dir { "" } else if is_last { "└── " } else { "├── " };
        let icon = if *is_dir { "\u{f07b}" } else { icon_for_name(name) };
        let content = format!("{}{} {}", branch, icon, name);

        let style = if *is_dir {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        lines.push(Line::from(Span::styled(
            format!("{}{}", prefix, content),
            style,
        )));

        let entry_path = path.join(name);

        if *is_dir {
            out_entries.push(PreviewEntry {
                name: name.clone(),
                full_path: entry_path.clone(),
                is_dir: true,
                line_index: *line_counter,
            });
        }

        *line_counter += 1;

        if *is_dir && current_depth < max_depth {
            let child_prefix = if is_last {
                format!("{}    ", prefix)
            } else {
                format!("{}│   ", prefix)
            };
            lines.extend(build_tree_with_entries(
                &entry_path,
                max_depth,
                current_depth + 1,
                &child_prefix,
                show_dotfiles,
                show_winhidden,
                out_entries,
                line_counter,
            ));
        }
    }

    lines
}

fn build_tree_lines(
    path: &Path,
    max_depth: usize,
    current_depth: usize,
    prefix: &str,
    show_dotfiles: bool,
    show_winhidden: bool,
) -> Vec<Line<'static>> {
    build_tree_with_entries(path, max_depth, current_depth, prefix, show_dotfiles, show_winhidden, &mut Vec::new(), &mut 0)
}

fn preview_directory(path: &Path, show_dotfiles: bool, show_winhidden: bool) -> Text<'static> {
    let mut lines: Vec<Line<'static>> = Vec::new();

    let tree = build_tree_lines(path, 2, 0, "", show_dotfiles, show_winhidden);
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

