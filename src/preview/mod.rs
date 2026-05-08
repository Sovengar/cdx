use std::fs;
use std::path::Path;

use ansi_to_tui::IntoText as _;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};

use crate::tui::app::{App, Mode};
use crate::walker::DirEntryItem;

pub fn generate(app: &App, item: &DirEntryItem) -> Text<'static> {
    if app.mode == Mode::Grep {
        return grep_preview(&item.full_path, &app.query);
    }

    let full_path = app.current_dir.join(&item.rel_path);

    if item.is_dir {
        preview_directory(&full_path)
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

fn preview_directory(path: &Path) -> Text<'static> {
    let mut lines: Vec<Line<'static>> = Vec::new();

    lines.push(header_line("=== CONTENTS ==="));

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
            Ok(text) => lines.extend(text.lines),
            Err(_) => lines.extend(fallback_read_dir(path)),
        },
        Err(_) => lines.extend(fallback_read_dir(path)),
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
