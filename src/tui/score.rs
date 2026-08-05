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
