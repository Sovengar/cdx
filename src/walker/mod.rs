use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use ignore::WalkBuilder;

use crate::config;

#[derive(Debug, Clone)]
pub struct DirEntryItem {
    pub display: String,
    pub rel_path: String,
    pub full_path: std::path::PathBuf,
    pub is_zoxide: bool,
    pub is_dir: bool,
}

pub fn should_exclude(name: &str, show_dotfiles: bool, show_winhidden: bool) -> bool {
    if config::get().exclude_dirs.contains(&name) {
        return true;
    }
    if !show_winhidden && config::get().exclude_win_dirs.contains(&name) {
        return true;
    }
    if !show_dotfiles && name.starts_with('.') {
        return true;
    }
    false
}

fn entry_filter(entry: &ignore::DirEntry, show_dotfiles: bool, show_winhidden: bool) -> bool {
    let name = entry.file_name().to_string_lossy();
    if should_exclude(&name, show_dotfiles, show_winhidden) {
        return false;
    }
    entry.file_type().map_or(false, |ft| ft.is_dir())
}

pub fn list_dirs(
    root: &Path,
    show_dotfiles: bool,
    show_winhidden: bool,
) -> Vec<DirEntryItem> {
    let mut entries: Vec<DirEntryItem> = Vec::new();
    let Ok(dir_entries) = std::fs::read_dir(root) else {
        return entries;
    };

    for entry in dir_entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if should_exclude(&name, show_dotfiles, show_winhidden) {
            continue;
        }
        if !entry.file_type().map_or(false, |ft| ft.is_dir()) {
            continue;
        }
        let full_path = entry.path();
        let full_str = full_path.to_string_lossy().replace('\\', "/");
        let root_str = root.to_string_lossy().replace('\\', "/");
        let rel = full_str
            .strip_prefix(&root_str)
            .unwrap_or(&full_str)
            .trim_start_matches('/')
            .to_string();
        entries.push(DirEntryItem {
            display: rel.clone(),
            rel_path: rel,
            full_path,
            is_zoxide: false,
            is_dir: true,
        });
    }

    entries.sort_by(|a, b| a.display.cmp(&b.display));
    entries
}

pub fn list_files(
    root: &Path,
    show_dotfiles: bool,
    show_winhidden: bool,
) -> Vec<DirEntryItem> {
    let mut builder = WalkBuilder::new(root);
    builder.max_depth(Some(config::get().max_secondary_depth));
    builder.hidden(!show_dotfiles);
    builder.require_git(false);
    builder.filter_entry(move |entry| {
        let name = entry.file_name().to_string_lossy();
        !should_exclude(&name, show_dotfiles, show_winhidden)
    });

    builder
        .build()
        .filter_map(|r| r.ok())
        .filter(|e| {
            e.path() != root && e.file_type().map_or(false, |ft| ft.is_file())
        })
        .map(|e| DirEntryItem {
            display: e.file_name().to_string_lossy().to_string(),
            rel_path: e.path().strip_prefix(root).unwrap().to_string_lossy().to_string(),
            full_path: e.path().to_path_buf(),
            is_zoxide: false,
            is_dir: false,
        })
        .collect()
}

pub fn recursive_dir_search(
    root: &Path,
    show_dotfiles: bool,
    show_winhidden: bool,
    cancel: &AtomicBool,
) -> Vec<DirEntryItem> {
    let mut builder = WalkBuilder::new(root);
    builder.max_depth(Some(config::get().max_secondary_depth));
    builder.hidden(!show_dotfiles);
    builder.require_git(false);
    builder.filter_entry(move |entry| entry_filter(entry, show_dotfiles, show_winhidden));

    collect_entries(&mut builder, root, Some(cancel))
}

fn collect_entries(
    builder: &mut WalkBuilder,
    root: &Path,
    cancel: Option<&AtomicBool>,
) -> Vec<DirEntryItem> {
    let mut entries: Vec<DirEntryItem> = Vec::new();
    let root_str = root.to_string_lossy().replace('\\', "/");

    for result in builder.build() {
        if cancel.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            break;
        }
        if let Ok(entry) = result {
            if entry.path() == root {
                continue;
            }
            let full_str = entry.path().to_string_lossy().replace('\\', "/");
            let rel = if let Some(stripped) = full_str.strip_prefix(&root_str) {
                stripped.trim_start_matches('/')
            } else {
                &full_str
            };
            entries.push(DirEntryItem {
                display: rel.to_string(),
                rel_path: rel.to_string(),
                full_path: entry.path().to_path_buf(),
                is_zoxide: false,
                is_dir: true,
            });
        }
    }

    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn test_root(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("cdx-{}-{}", name, std::process::id()))
    }

    fn cleanup(root: &std::path::Path) {
        let _ = fs::remove_dir_all(root);
    }

    // ── list_dirs ───────────────────────────────────────────────

    #[test]
    fn list_dirs_does_not_apply_global_ignore_rules() {
        config::init();
        let root = test_root("ignore");
        cleanup(&root);
        fs::create_dir_all(root.join("Dropbox")).unwrap();
        fs::create_dir_all(root.join(".local")).unwrap();
        fs::write(root.join(".ignore"), "Dropbox/\n.local/\n").unwrap();

        let entries = list_dirs(&root, false, false);

        assert!(entries.iter().any(|entry| entry.display == "Dropbox"));
        assert!(!entries.iter().any(|entry| entry.display == ".local"));
        cleanup(&root);
    }

    #[test]
    fn list_dirs_respects_exclude_dirs() {
        config::init();
        let root = test_root("exclude");
        cleanup(&root);
        fs::create_dir_all(root.join("node_modules")).unwrap();
        fs::create_dir_all(root.join("projects")).unwrap();
        fs::create_dir_all(root.join(".git")).unwrap();

        let entries = list_dirs(&root, true, true);
        let names: Vec<&str> = entries.iter().map(|e| e.display.as_str()).collect();

        assert!(!names.contains(&"node_modules"), "node_modules should be excluded");
        assert!(!names.contains(&".git"), ".git should be excluded");
        assert!(names.contains(&"projects"), "projects should be present");
        cleanup(&root);
    }

    #[test]
    fn list_dirs_respects_show_dotfiles_false() {
        config::init();
        let root = test_root("dotfiles");
        cleanup(&root);
        fs::create_dir_all(root.join(".config")).unwrap();
        fs::create_dir_all(root.join("visible")).unwrap();

        let entries = list_dirs(&root, false, true);
        let names: Vec<&str> = entries.iter().map(|e| e.display.as_str()).collect();
        assert!(!names.contains(&".config"), ".config hidden when show_dotfiles=false");
        assert!(names.contains(&"visible"));

        let entries = list_dirs(&root, true, true);
        let names: Vec<&str> = entries.iter().map(|e| e.display.as_str()).collect();
        assert!(names.contains(&".config"), ".config shown when show_dotfiles=true");
        cleanup(&root);
    }

    #[test]
    fn list_dirs_sorted_alphabetically() {
        config::init();
        let root = test_root("sort");
        cleanup(&root);
        fs::create_dir_all(root.join("zebra")).unwrap();
        fs::create_dir_all(root.join("alpha")).unwrap();
        fs::create_dir_all(root.join("beta")).unwrap();

        let entries = list_dirs(&root, true, true);
        let names: Vec<&str> = entries.iter().map(|e| e.display.as_str()).collect();
        assert_eq!(names, vec!["alpha", "beta", "zebra"]);
        cleanup(&root);
    }

    #[test]
    fn list_dirs_only_returns_directories() {
        config::init();
        let root = test_root("dirs-only");
        cleanup(&root);
        fs::create_dir_all(root.join("adir")).unwrap();
        fs::write(root.join("afile.txt"), "content").unwrap();

        let entries = list_dirs(&root, true, true);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].display, "adir");
        assert!(entries[0].is_dir);
        cleanup(&root);
    }

    #[test]
    fn list_dirs_empty_dir() {
        config::init();
        let root = test_root("empty");
        cleanup(&root);
        fs::create_dir_all(&root).unwrap();

        let entries = list_dirs(&root, true, true);
        assert!(entries.is_empty());
        cleanup(&root);
    }

    #[test]
    fn list_dirs_nonexistent_dir() {
        config::init();
        let root = test_root("nonexistent");
        let entries = list_dirs(&root, true, true);
        assert!(entries.is_empty());
    }

    // ── list_files ──────────────────────────────────────────────

    #[test]
    fn list_files_only_returns_files() {
        config::init();
        let root = test_root("files-only");
        cleanup(&root);
        fs::create_dir_all(root.join("subdir")).unwrap();
        fs::write(root.join("file.txt"), "content").unwrap();
        fs::write(root.join("subdir").join("nested.rs"), "code").unwrap();

        let entries = list_files(&root, true, true);
        assert!(entries.iter().all(|e| !e.is_dir), "list_files should only return files");
        assert!(entries.iter().any(|e| e.display == "file.txt"));
        assert!(entries.iter().any(|e| e.display == "nested.rs"));
        cleanup(&root);
    }

    #[test]
    fn list_files_display_is_filename_only() {
        config::init();
        let root = test_root("filename");
        cleanup(&root);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src").join("main.rs"), "fn main() {}").unwrap();

        let entries = list_files(&root, true, true);
        let main_entry = entries.iter().find(|e| e.display == "main.rs").unwrap();
        assert_eq!(main_entry.display, "main.rs");
        assert!(main_entry.full_path.to_string_lossy().contains("src"));
        cleanup(&root);
    }

    #[test]
    fn list_files_respects_exclude_dirs() {
        config::init();
        let root = test_root("files-exclude");
        cleanup(&root);
        fs::create_dir_all(root.join("node_modules")).unwrap();
        fs::write(root.join("node_modules").join("pkg.js"), "code").unwrap();
        fs::write(root.join("app.js"), "code").unwrap();

        let entries = list_files(&root, true, true);
        assert!(entries.iter().any(|e| e.display == "app.js"));
        assert!(!entries.iter().any(|e| e.display == "pkg.js"));
        cleanup(&root);
    }

    // ── recursive_dir_search ────────────────────────────────────

    #[test]
    fn recursive_dir_search_finds_nested_dirs() {
        config::init();
        let root = test_root("recursive");
        cleanup(&root);
        fs::create_dir_all(root.join("a").join("b").join("c")).unwrap();
        fs::create_dir_all(root.join("x")).unwrap();

        let cancel = AtomicBool::new(false);
        let entries = recursive_dir_search(&root, true, true, &cancel);
        let displays: Vec<&str> = entries.iter().map(|e| e.display.as_str()).collect();
        assert!(displays.contains(&"a"));
        assert!(displays.contains(&"a/b"));
        assert!(displays.contains(&"a/b/c"));
        assert!(displays.contains(&"x"));
        cleanup(&root);
    }

    #[test]
    fn recursive_dir_search_cancellation() {
        config::init();
        let root = test_root("cancel");
        cleanup(&root);
        for i in 0..50 {
            fs::create_dir_all(root.join(format!("dir_{}", i))).unwrap();
        }

        let cancel = AtomicBool::new(true); // pre-cancelled
        let entries = recursive_dir_search(&root, true, true, &cancel);
        assert!(entries.is_empty(), "cancelled search should return empty");
        cleanup(&root);
    }

    #[test]
    fn recursive_dir_search_excludes_filtered_dirs() {
        config::init();
        let root = test_root("recurse-exclude");
        cleanup(&root);
        fs::create_dir_all(root.join("target").join("inner")).unwrap();
        fs::create_dir_all(root.join("src")).unwrap();

        let cancel = AtomicBool::new(false);
        let entries = recursive_dir_search(&root, true, true, &cancel);
        let displays: Vec<&str> = entries.iter().map(|e| e.display.as_str()).collect();
        assert!(!displays.iter().any(|d| d.starts_with("target")), "target should be excluded");
        assert!(displays.contains(&"src"));
        cleanup(&root);
    }

    #[test]
    fn recursive_dir_search_respects_dotfiles() {
        config::init();
        let root = test_root("recurse-dotfiles");
        cleanup(&root);
        fs::create_dir_all(root.join(".hidden_dir")).unwrap();
        fs::create_dir_all(root.join("visible_dir")).unwrap();

        let cancel = AtomicBool::new(false);

        let entries = recursive_dir_search(&root, false, true, &cancel);
        let displays: Vec<&str> = entries.iter().map(|e| e.display.as_str()).collect();
        assert!(!displays.contains(&".hidden_dir"), "dotfiles hidden");
        assert!(displays.contains(&"visible_dir"));

        let entries = recursive_dir_search(&root, true, true, &cancel);
        let displays: Vec<&str> = entries.iter().map(|e| e.display.as_str()).collect();
        assert!(displays.contains(&".hidden_dir"), "dotfiles shown");
        cleanup(&root);
    }

    // ── should_exclude ──────────────────────────────────────────

    #[test]
    fn should_exclude_exclude_dirs() {
        config::init();
        assert!(should_exclude("node_modules", true, true));
        assert!(should_exclude(".git", true, true));
        assert!(should_exclude("target", true, true));
    }

    #[test]
    fn should_exclude_win_dirs() {
        config::init();
        assert!(should_exclude("AppData", true, false));
        assert!(!should_exclude("AppData", true, true));
    }

    #[test]
    fn should_exclude_dotfiles() {
        config::init();
        assert!(should_exclude(".config", false, true));
        assert!(!should_exclude(".config", true, true));
    }

    #[test]
    fn should_not_exclude_normal_dirs() {
        config::init();
        assert!(!should_exclude("projects", true, true));
        assert!(!should_exclude("src", true, true));
    }
}
