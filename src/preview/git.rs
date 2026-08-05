use std::path::Path;

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

pub fn git_status_lines(path: &Path) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();

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

    lines
}

pub struct GitInfo {
    pub toplevel: String,
    pub branch: Option<String>,
    pub dirty: bool,
}

pub fn git_info(dir: &Path) -> Option<GitInfo> {
    let toplevel = std::process::Command::new("git")
        .args(["-C", &dir.to_string_lossy(), "rev-parse", "--show-toplevel"])
        .output()
        .ok()?;

    if !toplevel.status.success() {
        return None;
    }

    let top = String::from_utf8_lossy(&toplevel.stdout).trim().to_string();

    let branch = std::process::Command::new("git")
        .args(["-C", &dir.to_string_lossy(), "rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        });

    let status = std::process::Command::new("git")
        .args(["-C", &dir.to_string_lossy(), "status", "--porcelain"])
        .output()
        .ok();
    let dirty = status.map_or(false, |o| !o.stdout.is_empty());

    Some(GitInfo {
        toplevel: top,
        branch,
        dirty,
    })
}

pub fn git_info_lines(dir: &Path, home: Option<&Path>) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();

    if let Some(info) = git_info(dir) {
        let display_path = display_path_with_home(&std::path::PathBuf::from(&info.toplevel), home);
        lines.push(Line::from(Span::styled(
            " git:".to_string(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            format!("  {}", display_path),
            Style::default().fg(Color::Cyan),
        )));
        if let Some(b) = info.branch {
            lines.push(Line::from(Span::styled(
                format!(
                    "  {} ({})",
                    b,
                    if info.dirty { "dirty" } else { "clean" }
                ),
                if info.dirty {
                    Style::default().fg(Color::LightRed)
                } else {
                    Style::default().fg(Color::LightGreen)
                },
            )));
        }
    }

    lines
}

fn display_path_with_home(path: &std::path::Path, home: Option<&Path>) -> String {
    let s = path.to_string_lossy().replace('\\', "/");
    if let Some(home_dir) = home {
        let home_str = home_dir.to_string_lossy().replace('\\', "/");
        if s.starts_with(&home_str) {
            return format!("~{}", &s[home_str.len()..]);
        }
    }
    s
}
