use std::fs;
use std::path::Path;

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::ui::icons::icon_for_name;

use super::PreviewEntry;

pub struct TreeContext<'a> {
    pub show_dotfiles: bool,
    pub show_winhidden: bool,
    pub out_entries: &'a mut Vec<PreviewEntry>,
    pub line_counter: &'a mut usize,
}

pub fn build_tree_lines(
    path: &Path,
    max_depth: usize,
    current_depth: usize,
    prefix: &str,
    ctx: &mut TreeContext<'_>,
) -> Vec<Line<'static>> {
    let mut items: Vec<(String, bool)> = Vec::new();

    if let Ok(dir_entries) = fs::read_dir(path) {
        for entry in dir_entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();

            if crate::walker::should_exclude(&name, ctx.show_dotfiles, ctx.show_winhidden) {
                continue;
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
        let branch = if current_depth == 0 && !*is_dir {
            ""
        } else if is_last {
            "└── "
        } else {
            "├── "
        };
        let icon = if *is_dir {
            "\u{f07b}"
        } else {
            icon_for_name(name)
        };
        let content = format!("{}{} {}", branch, icon, name);

        let style = if *is_dir {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        lines.push(Line::from(Span::styled(
            format!("{}{}", prefix, content),
            style,
        )));

        let entry_path = path.join(name);

        if *is_dir {
            ctx.out_entries.push(PreviewEntry {
                name: name.clone(),
                full_path: entry_path.clone(),
                is_dir: true,
                line_index: *ctx.line_counter,
            });
        }

        *ctx.line_counter += 1;

        if *is_dir && current_depth < max_depth {
            let child_prefix = if is_last {
                format!("{}    ", prefix)
            } else {
                format!("{}│   ", prefix)
            };
            lines.extend(build_tree_lines(
                &entry_path,
                max_depth,
                current_depth + 1,
                &child_prefix,
                ctx,
            ));
        }
    }

    lines
}
