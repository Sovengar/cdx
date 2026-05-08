use std::fs;
use std::path::{Path, PathBuf};

use ansi_to_tui::IntoText as _;
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
        return grep_preview(&item.full_path, &app.query);
    }

    let full_path = app.current_dir.join(&item.rel_path);

    if item.is_dir {
        preview_directory(&full_path, app.show_dotfiles, app.show_winhidden)
    } else {
        preview_file(&full_path)
    }
}

fn icon_for_name(name: &str) -> &'static str {
    if name.starts_with('.') {
        return "\u{e65d}";
    }
    match Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
    {
        "rs" => "\u{eba8}",
        "toml" | "yml" | "yaml" | "json" => "\u{eb01}",
        "md" | "txt" => "\u{f15c}",
        "ps1" | "sh" | "bat" | "cmd" => "\u{ebc7}",
        "exe" | "dll" => "\u{f17a}",
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "ico" => "\u{f03e}",
        "gitignore" | "gitattributes" | "gitmodules" => "\u{e65d}",
        _ => "\u{f15b}",
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
        for entry in &mut entries {
            entry.line_index += 1;
        }
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

            if crate::config::EXCLUDE_DIRS.contains(&name.as_str()) {
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

    lines.push(header_line("=== TREE ==="));
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

    match std::process::Command::new("git")
        .args(["-C"])
        .arg(path)
        .args(["status", "--short"])
        .output()
    {
        Ok(out) => {
            let status = String::from_utf8_lossy(&out.stdout);
            if status.trim().is_empty() {
                lines.push(Line::from(Span::styled(
                    "Clean".to_string(),
                    Style::default().fg(Color::DarkGray),
                )));
            } else {
                for line in status.lines() {
                    let style = if line.starts_with("??")
                        || line.starts_with(" M")
                        || line.starts_with("A ")
                        || line.starts_with("D ")
                    {
                        Style::default().fg(Color::LightRed)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    lines.push(Line::from(Span::styled(line.to_string(), style)));
                }
            }
        }
        Err(_) => {
            lines.push(Line::from(Span::styled(
                "(not a git repo)".to_string(),
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    Text::from(lines)
}

pub fn directory_contents(path: &Path) -> Text<'static> {
    match std::process::Command::new("eza")
        .args([
            "--icons=always",
            "--color=always",
            "--group-directories-first",
            "--grid",
            "--width=40",
        ])
        .arg(path)
        .output()
    {
        Ok(out) => match out.stdout.into_text() {
            Ok(text) => text,
            Err(_) => Text::from(fallback_read_dir(path)),
        },
        Err(_) => Text::from(fallback_read_dir(path)),
    }
}

fn fallback_read_dir(path: &Path) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();

    if let Ok(entries) = fs::read_dir(path) {
        let mut dirs: Vec<String> = Vec::new();
        let mut files: Vec<String> = Vec::new();

        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            if entry.file_type().map_or(false, |t| t.is_dir()) {
                dirs.push(name);
            } else {
                files.push(name);
            }
        }

        dirs.sort();
        files.sort();

        for d in &dirs {
            lines.push(Line::from(Span::styled(
                format!("\u{f07b} {}", d),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )));
        }
        for f in &files {
            let icon = icon_for_name(f);
            lines.push(Line::from(Span::styled(
                format!("{} {}", icon, f),
                Style::default().fg(Color::White),
            )));
        }
    }

    lines
}

fn preview_file(path: &Path) -> Text<'static> {
    match std::process::Command::new("bat")
        .args([
            "--color=always",
            "--line-range",
            ":50",
            "--paging=never",
        ])
        .arg(path)
        .output()
    {
        Ok(out) => match out.stdout.into_text() {
            Ok(text) => return text,
            Err(_) => {}
        },
        Err(_) => {}
    }

    match fs::read_to_string(path) {
        Ok(content) => {
            let lines: Vec<Line<'static>> = content
                .lines()
                .take(50)
                .map(|l| Line::from(Span::styled(l.to_string(), Style::default().fg(Color::White))))
                .collect();
            Text::from(lines)
        }
        Err(_) => Text::default(),
    }
}

fn grep_preview(file_path: &Path, query: &str) -> Text<'static> {
    let output = std::process::Command::new("rg")
        .args(["--context=2", "--color=never", "--max-count", "50"])
        .arg(query)
        .arg(file_path)
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    if output.is_empty() {
        let line = Line::from(Span::styled(
            format!(" (no matches in {})", file_path.display()),
            Style::default().fg(Color::DarkGray),
        ));
        return Text::from(line);
    }

    let lines: Vec<Line<'static>> = output
        .lines()
        .map(|line| {
            let style = if line.starts_with("--") {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default().fg(Color::White)
            };
            Line::from(Span::styled(line.to_string(), style))
        })
        .collect();

    Text::from(lines)
}
