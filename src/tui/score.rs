use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Matcher, Utf32Str};

use crate::walker::DirEntryItem;

pub fn score_items(
    query: &str,
    items: &[DirEntryItem],
    matcher: &mut Matcher,
    scratch: &mut Vec<char>,
) -> Vec<(usize, u32)> {
    let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);

    let mut scored: Vec<(usize, u32)> = items
        .iter()
        .enumerate()
        .filter_map(|(i, item)| {
            scratch.clear();
            let haystack = Utf32Str::new(item.display.as_str(), scratch);
            pattern.score(haystack, matcher).map(|s| (i, s))
        })
        .collect();

    scored.sort_by(|a, b| b.1.cmp(&a.1));
    scored
}

#[cfg(test)]
mod tests {
    use super::*;
    use nucleo_matcher::Config as MatcherConfig;

    fn make_item(display: &str) -> DirEntryItem {
        DirEntryItem {
            display: display.to_string(),
            rel_path: display.to_string(),
            full_path: std::path::PathBuf::from(display),
            is_zoxide: false,
            is_dir: false,
        }
    }

    fn setup() -> (Matcher, Vec<char>) {
        (Matcher::new(MatcherConfig::DEFAULT.match_paths()), Vec::new())
    }

    #[test]
    fn test_score_exact_match_ranks_higher_than_substring() {
        let (mut matcher, mut scratch) = setup();
        let items = vec![
            make_item("project-alpha"),
            make_item("my-project"),
            make_item("project"),
        ];
        let scored = score_items("project", &items, &mut matcher, &mut scratch);
        assert_eq!(scored.len(), 3, "all three should match");
        let exact_idx = scored.iter().position(|(i, _)| items[*i].display == "project").unwrap();
        assert!(exact_idx <= 1, "exact match should be in top 2 positions");
    }

    #[test]
    fn test_score_case_insensitive() {
        let (mut matcher, mut scratch) = setup();
        let items = vec![make_item("README.md")];
        let scored = score_items("readme", &items, &mut matcher, &mut scratch);
        assert_eq!(scored.len(), 1, "should match case-insensitively");
    }

    #[test]
    fn test_score_no_match_returns_empty() {
        let (mut matcher, mut scratch) = setup();
        let items = vec![make_item("hello")];
        let scored = score_items("xyz", &items, &mut matcher, &mut scratch);
        assert!(scored.is_empty());
    }

    #[test]
    fn test_score_empty_query_returns_all_or_empty() {
        let (mut matcher, mut scratch) = setup();
        let items = vec![make_item("hello")];
        let scored = score_items("", &items, &mut matcher, &mut scratch);
        // nucleo behavior with empty query varies - just ensure no panic
        assert!(scored.len() <= items.len());
    }

    #[test]
    fn test_score_sorted_by_relevance() {
        let (mut matcher, mut scratch) = setup();
        let items = vec![
            make_item("config.json"),
            make_item("configure"),
            make_item("config"),
        ];
        let scored = score_items("config", &items, &mut matcher, &mut scratch);
        // All should match, sorted by score descending
        assert_eq!(scored.len(), 3);
        assert!(scored[0].1 >= scored[1].1);
        assert!(scored[1].1 >= scored[2].1);
    }

    #[test]
    fn test_score_returns_original_indices() {
        let (mut matcher, mut scratch) = setup();
        let items = vec![
            make_item("alpha"),
            make_item("beta"),
            make_item("gamma"),
        ];
        let scored = score_items("a", &items, &mut matcher, &mut scratch);
        let indices: Vec<usize> = scored.iter().map(|(i, _)| *i).collect();
        assert!(indices.contains(&0), "alpha (index 0) should match");
    }

    #[test]
    fn test_score_substring_match() {
        let (mut matcher, mut scratch) = setup();
        let items = vec![make_item("my-project-name")];
        let scored = score_items("proj", &items, &mut matcher, &mut scratch);
        assert_eq!(scored.len(), 1, "should match substring");
    }
}
