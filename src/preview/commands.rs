use std::fs;
use std::path::Path;

use ansi_to_tui::IntoText as _;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use regex::Regex;

use crate::ui::icons::icon_for_name;

const EXACT_MATCH_ANSI: &str = "\x1b[1;43m";
const FOLDED_MATCH_ANSI: &str = "\x1b[1;46m";
const RESET_ANSI: &str = "\x1b[0m";

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

pub fn preview_file(path: &Path) -> Text<'static> {
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

pub fn grep_preview(file_path: &Path, query: &str) -> Text<'static> {
    let bat_output = match std::process::Command::new("bat")
        .args([
            "--color=always",
            "--style=numbers",
            "--paging=never",
            "--line-range",
            ":200",
            "--theme",
            "RGX Muted",
        ])
        .arg(file_path)
        .output()
    {
        Ok(out) if out.status.success() && !out.stdout.is_empty() => out.stdout,
        _ => return fallback_grep_preview(file_path, query),
    };

    let highlighted = highlight_matches(&bat_output, query);

    match highlighted.into_text() {
        Ok(text) => text,
        Err(_) => fallback_grep_preview(file_path, query),
    }
}

fn highlight_matches(ansi_bytes: &[u8], query: &str) -> Vec<u8> {
    if query.is_empty() {
        return ansi_bytes.to_vec();
    }

    let re = match Regex::new(&format!("(?i){}", regex::escape(query))) {
        Ok(re) => re,
        Err(_) => return ansi_bytes.to_vec(),
    };

    let content = String::from_utf8_lossy(ansi_bytes);
    let mut result = Vec::with_capacity(ansi_bytes.len());

    for line in content.lines() {
        let mut highlighted = String::new();
        let mut last_end = 0;

        for mat in re.find_iter(line) {
            highlighted.push_str(&line[last_end..mat.start()]);

            let color = if mat.as_str().eq_ignore_ascii_case(query) {
                EXACT_MATCH_ANSI
            } else {
                FOLDED_MATCH_ANSI
            };

            highlighted.push_str(color);
            highlighted.push_str(mat.as_str());
            highlighted.push_str(RESET_ANSI);

            last_end = mat.end();
        }

        highlighted.push_str(&line[last_end..]);
        result.extend_from_slice(highlighted.as_bytes());
        result.push(b'\n');
    }

    result
}

fn fallback_grep_preview(file_path: &Path, query: &str) -> Text<'static> {
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
