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

#[cfg(test)]
mod tests {
    use super::*;

    fn home() -> PathBuf {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"))
    }

    fn home_str() -> String {
        home().to_string_lossy().replace('\\', "/")
    }

    fn make_zoxide_item(path: &str) -> PathBuf {
        PathBuf::from(path)
    }

    fn make_walker_item(name: &str, path: &str) -> DirEntryItem {
        DirEntryItem {
            display: name.to_string(),
            rel_path: name.to_string(),
            full_path: PathBuf::from(path),
            is_zoxide: false,
            is_dir: true,
        }
    }

    #[test]
    fn test_merge_limits_zoxide_entries() {
        let h = home_str();
        let cache = vec![
            make_zoxide_item(&format!("{}/a", h)),
            make_zoxide_item(&format!("{}/b", h)),
            make_zoxide_item(&format!("{}/c", h)),
        ];
        let current = home();
        let result = merge_with_dirs(&cache, vec![], &current, 2);
        let zoxide_count = result.iter().filter(|i| i.is_zoxide).count();
        assert!(zoxide_count <= 2, "at most 2 zoxide entries");
    }

    #[test]
    fn test_merge_excludes_current_dir() {
        let current = home();
        let cache = vec![make_zoxide_item(&current.to_string_lossy())];
        let result = merge_with_dirs(&cache, vec![], &current, 5);
        assert!(result.is_empty(), "current dir should be excluded from zoxide");
    }

    #[test]
    fn test_merge_excludes_duplicates_with_walker() {
        let h = home_str();
        let current = home();
        let cache = vec![make_zoxide_item(&format!("{}/projects", h))];
        let walker = vec![make_walker_item("projects", &format!("{}/projects", h))];
        let result = merge_with_dirs(&cache, walker, &current, 5);
        let count = result.iter().filter(|i| i.display == "projects").count();
        assert_eq!(count, 1, "should not duplicate dirs already in walker");
    }

    #[test]
    fn test_merge_zoxide_appears_first() {
        let h = home_str();
        let current = home();
        let cache = vec![make_zoxide_item(&format!("{}/zz-top", h))];
        let walker = vec![make_walker_item("aaa", &format!("{}/aaa", h))];
        let result = merge_with_dirs(&cache, walker, &current, 5);
        assert_eq!(result.len(), 2);
        assert!(result[0].is_zoxide, "zoxide should come first");
        assert!(!result[1].is_zoxide);
    }

    #[test]
    fn test_merge_empty_cache_only_walker() {
        let h = home_str();
        let current = home();
        let walker = vec![
            make_walker_item("a", &format!("{}/a", h)),
            make_walker_item("b", &format!("{}/b", h)),
        ];
        let result = merge_with_dirs(&[], walker, &current, 5);
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|i| !i.is_zoxide));
    }

    #[test]
    fn test_merge_empty_walker_only_zoxide() {
        let h = home_str();
        let current = home();
        let cache = vec![make_zoxide_item(&format!("{}/config", h))];
        let result = merge_with_dirs(&cache, vec![], &current, 5);
        assert_eq!(result.len(), 1);
        assert!(result[0].is_zoxide);
    }

    #[test]
    fn test_merge_all_empty() {
        let current = home();
        let result = merge_with_dirs(&[], vec![], &current, 5);
        assert!(result.is_empty());
    }

    #[test]
    fn test_merge_walker_items_always_included() {
        let h = home_str();
        let current = home();
        let cache = vec![];
        let walker = vec![
            make_walker_item("alpha", &format!("{}/alpha", h)),
            make_walker_item("beta", &format!("{}/beta", h)),
        ];
        let result = merge_with_dirs(&cache, walker, &current, 5);
        assert_eq!(result.len(), 2);
        assert!(result.iter().any(|i| i.display == "alpha"));
        assert!(result.iter().any(|i| i.display == "beta"));
    }

    #[test]
    fn test_merge_walker_items_preserved_when_zoxide_exhausts_limit() {
        let h = home_str();
        let current = home();
        let cache = vec![
            make_zoxide_item(&format!("{}/z1", h)),
            make_zoxide_item(&format!("{}/z2", h)),
        ];
        let walker = vec![
            make_walker_item("w1", &format!("{}/w1", h)),
            make_walker_item("w2", &format!("{}/w2", h)),
        ];
        let result = merge_with_dirs(&cache, walker, &current, 1);
        let zoxide_count = result.iter().filter(|i| i.is_zoxide).count();
        let walker_count = result.iter().filter(|i| !i.is_zoxide).count();
        assert_eq!(zoxide_count, 1, "only 1 zoxide due to limit");
        assert_eq!(walker_count, 2, "all walker items preserved");
    }
}
