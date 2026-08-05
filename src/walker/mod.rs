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

    #[test]
    fn list_dirs_does_not_apply_global_ignore_rules() {
        config::init();
        let root = std::env::temp_dir().join(format!("cdx-walker-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("Dropbox")).unwrap();
        fs::create_dir_all(root.join(".local")).unwrap();
        fs::write(root.join(".ignore"), "Dropbox/\n.local/\n").unwrap();

        let entries = list_dirs(&root, false, false);

        assert!(entries.iter().any(|entry| entry.display == "Dropbox"));
        assert!(!entries.iter().any(|entry| entry.display == ".local"));
        fs::remove_dir_all(root).unwrap();
    }
}
