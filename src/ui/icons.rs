use std::path::Path;

use ratatui::style::{Color, Modifier, Style};

use crate::walker::DirEntryItem;

pub fn icon_for_item(item: &DirEntryItem) -> &'static str {
    if item.is_zoxide {
        return "★";
    }
    if item.is_dir {
        return "";
    }
    icon_for_name(&item.display)
}

pub fn style_for_item(item: &DirEntryItem) -> Style {
    if item.is_zoxide {
        return Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
    }
    if item.is_dir {
        return Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
    }
    style_for_extension(&item.display)
}

pub fn icon_for_name(name: &str) -> &'static str {
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

pub fn style_for_extension(name: &str) -> Style {
    match Path::new(name)
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
