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

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Modifier;

    fn make_item(display: &str, is_zoxide: bool, is_dir: bool) -> DirEntryItem {
        DirEntryItem {
            display: display.to_string(),
            rel_path: display.to_string(),
            full_path: std::path::PathBuf::from(display),
            is_zoxide,
            is_dir,
        }
    }

    // ── icon_for_item ───────────────────────────────────────────

    #[test]
    fn test_icon_zoxide_is_star() {
        let item = make_item("path", true, true);
        assert_eq!(icon_for_item(&item), "★");
    }

    #[test]
    fn test_icon_dir_is_empty_string() {
        let item = make_item("projects", false, true);
        assert_eq!(icon_for_item(&item), "");
    }

    #[test]
    fn test_icon_rs_file() {
        let item = make_item("main.rs", false, false);
        let icon = icon_for_item(&item);
        assert_eq!(icon, "\u{eba8}");
    }

    #[test]
    fn test_icon_toml_file() {
        let item = make_item("config.toml", false, false);
        assert_eq!(icon_for_item(&item), "\u{eb01}");
    }

    #[test]
    fn test_icon_hidden_file() {
        let item = make_item(".gitignore", false, false);
        assert_eq!(icon_for_item(&item), "\u{e65d}");
    }

    #[test]
    fn test_icon_unknown_extension() {
        let item = make_item("unknown.xyz", false, false);
        assert_eq!(icon_for_item(&item), "\u{f15b}");
    }

    // ── icon_for_name ───────────────────────────────────────────

    #[test]
    fn test_icon_for_name_rs() {
        assert_eq!(icon_for_name("foo.rs"), "\u{eba8}");
    }

    #[test]
    fn test_icon_for_name_hidden() {
        assert_eq!(icon_for_name(".env"), "\u{e65d}");
    }

    #[test]
    fn test_icon_for_name_shell() {
        assert_eq!(icon_for_name("install.sh"), "\u{ebc7}");
    }

    #[test]
    fn test_icon_for_name_image() {
        assert_eq!(icon_for_name("logo.png"), "\u{f03e}");
    }

    // ── style_for_item ──────────────────────────────────────────

    #[test]
    fn test_style_zoxide_yellow_bold() {
        let item = make_item("path", true, true);
        let style = style_for_item(&item);
        assert_eq!(style.fg, Some(Color::Yellow));
        assert!(style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_style_dir_cyan_bold() {
        let item = make_item("projects", false, true);
        let style = style_for_item(&item);
        assert_eq!(style.fg, Some(Color::Cyan));
        assert!(style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_style_rs_file_light_blue() {
        let item = make_item("main.rs", false, false);
        let style = style_for_item(&item);
        assert_eq!(style.fg, Some(Color::LightBlue));
    }

    #[test]
    fn test_style_toml_file_light_red() {
        let item = make_item("config.toml", false, false);
        let style = style_for_item(&item);
        assert_eq!(style.fg, Some(Color::LightRed));
    }

    // ── style_for_extension ─────────────────────────────────────

    #[test]
    fn test_style_for_extension_rs() {
        assert_eq!(style_for_extension("x.rs").fg, Some(Color::LightBlue));
    }

    #[test]
    fn test_style_for_extension_md() {
        assert_eq!(style_for_extension("x.md").fg, Some(Color::LightYellow));
    }

    #[test]
    fn test_style_for_extension_sh() {
        assert_eq!(style_for_extension("x.sh").fg, Some(Color::LightGreen));
    }

    #[test]
    fn test_style_for_extension_unknown() {
        assert_eq!(style_for_extension("x.xyz").fg, Some(Color::White));
    }
}
