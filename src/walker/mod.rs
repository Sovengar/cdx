use std::path::Path;

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

fn entry_filter(entry: &ignore::DirEntry, show_dotfiles: bool, show_winhidden: bool) -> bool {
    let name = entry.file_name().to_string_lossy();
    if config::get().exclude_dirs.contains(&name.as_ref()) {
        return false;
    }
    if !show_winhidden && config::get().exclude_win_dirs.contains(&name.as_ref()) {
        return false;
    }
    if !show_dotfiles && name.starts_with('.') {
        return false;
    }
    entry.file_type().map_or(false, |ft| ft.is_dir())
}

pub fn list_dirs(
    root: &Path,
    show_dotfiles: bool,
    show_winhidden: bool,
) -> Vec<DirEntryItem> {
    let mut builder = WalkBuilder::new(root);
    builder.max_depth(Some(1));
    builder.hidden(!show_dotfiles);
    builder.require_git(false);
    builder.filter_entry(move |entry| entry_filter(entry, show_dotfiles, show_winhidden));

    collect_entries(&mut builder, root)
}

pub fn list_files(
    root: &Path,
    show_dotfiles: bool,
    show_winhidden: bool,
) -> Vec<DirEntryItem> {
    let mut builder = WalkBuilder::new(root);
    builder.max_depth(Some(1));
    builder.hidden(!show_dotfiles);
    builder.require_git(false);
    builder.filter_entry(move |entry| {
        let name = entry.file_name().to_string_lossy();
        if config::get().exclude_dirs.contains(&name.as_ref()) {
            return false;
        }
        if !show_winhidden && config::get().exclude_win_dirs.contains(&name.as_ref()) {
            return false;
        }
        if !show_dotfiles && name.starts_with('.') {
            return false;
        }
        true
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
) -> Vec<DirEntryItem> {
    let mut builder = WalkBuilder::new(root);
    builder.max_depth(Some(config::get().max_secondary_depth));
    builder.hidden(!show_dotfiles);
    builder.require_git(false);
    builder.filter_entry(move |entry| entry_filter(entry, show_dotfiles, show_winhidden));

    collect_entries(&mut builder, root)
}

fn collect_entries(builder: &mut WalkBuilder, root: &Path) -> Vec<DirEntryItem> {
    let mut entries: Vec<DirEntryItem> = Vec::new();
    let root_str = root.to_string_lossy().replace('\\', "/");

    for result in builder.build() {
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
