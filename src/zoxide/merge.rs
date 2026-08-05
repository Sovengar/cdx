use std::path::PathBuf;

use crate::walker::DirEntryItem;

pub fn merge_with_dirs(
    zoxide_cache: &[PathBuf],
    walker_items: Vec<DirEntryItem>,
    current_dir: &PathBuf,
    limit: usize,
) -> Vec<DirEntryItem> {
    let home = dirs::home_dir();
    let home_str = home.as_ref().map(|h| h.to_string_lossy().replace('\\', "/"));
    let home_lower = home_str.as_ref().map(|s| s.to_lowercase());
    let mut zoxide_items: Vec<DirEntryItem> = Vec::new();

    for zpath in zoxide_cache {
        let full_str = zpath.to_string_lossy().replace('\\', "/");
        if let Some(ref hl) = home_lower {
            if !full_str.to_lowercase().starts_with(hl) {
                continue;
            }
        }
        let exists = walker_items.iter().any(|w| w.full_path == *zpath);
        let is_current = *zpath == *current_dir;
        if exists || is_current {
            continue;
        }
        let display = if let Some(ref hs) = home_str {
            if full_str.starts_with(hs.as_str()) {
                format!("~{}", &full_str[hs.len()..])
            } else {
                full_str.clone()
            }
        } else {
            full_str.clone()
        };
        zoxide_items.push(DirEntryItem {
            display,
            rel_path: zpath.to_string_lossy().replace('\\', "/"),
            full_path: zpath.clone(),
            is_zoxide: true,
            is_dir: true,
        });
        if zoxide_items.len() >= limit {
            break;
        }
    }

    let mut result = zoxide_items;
    let mut remaining = walker_items;
    remaining.retain(|w| !result.iter().any(|z| z.full_path == w.full_path));
    result.extend(remaining);
    result
}
