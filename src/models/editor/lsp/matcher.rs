//! Purpose: `IntelliJ` IDEA-style completion matching and ranking (exact prefix,
//! case-insensitive prefix, `CamelHumps` / `snake_case` word boundaries, and substring).

mod humps;

use crate::lsp::features::Completion;
use humps::{camel_humps, chars_eq, Scratch};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum MatchTier {
    ExactCasePrefix,
    IgnoreCasePrefix,
    CamelHumps,
    ExactCaseSubstring,
    IgnoreCaseSubstring,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct MatchRank {
    tier: MatchTier,
    penalty: usize,
    original_index: usize,
}

struct PreparedPrefix<'a> {
    raw: &'a str,
    char_len: usize,
    hump_query: Vec<char>,
}

impl<'a> PreparedPrefix<'a> {
    fn new(raw: &'a str) -> Self {
        Self {
            raw,
            char_len: raw.chars().count(),
            hump_query: raw.chars().filter(|&ch| ch != '_').collect(),
        }
    }
}

pub fn filter_and_rank(items: &[Completion], prefix: &str) -> Vec<Completion> {
    if prefix.is_empty() {
        return items.to_vec();
    }
    let prepared = PreparedPrefix::new(prefix);
    let mut scratch = Scratch::default();
    let mut ranks: Vec<MatchRank> = items
        .iter()
        .enumerate()
        .filter_map(|(original_index, item)| {
            let (tier, penalty) = score(&item.label, &prepared, &mut scratch)
                .or_else(|| score(&item.insert_text, &prepared, &mut scratch))?;
            Some(MatchRank {
                tier,
                penalty,
                original_index,
            })
        })
        .collect();
    ranks.sort_unstable();
    ranks
        .into_iter()
        .filter_map(|rank| items.get(rank.original_index).cloned())
        .collect()
}

fn score(
    candidate: &str,
    prefix: &PreparedPrefix<'_>,
    scratch: &mut Scratch,
) -> Option<(MatchTier, usize)> {
    let stem = identifier_stem(candidate);
    if stem.is_empty() {
        return None;
    }
    if stem.starts_with(prefix.raw) {
        return Some((MatchTier::ExactCasePrefix, usize::from(stem != prefix.raw)));
    }
    if starts_with_ignore_case(stem, prefix.raw) {
        let exact_len = stem.chars().count() == prefix.char_len;
        return Some((MatchTier::IgnoreCasePrefix, usize::from(!exact_len)));
    }
    if let Some(skipped) = camel_humps(stem, &prefix.hump_query, scratch) {
        return Some((MatchTier::CamelHumps, skipped));
    }
    if let Some(pos) = stem.find(prefix.raw) {
        return Some((MatchTier::ExactCaseSubstring, pos));
    }
    crate::casefold::find_ignore_case(stem, prefix.raw)
        .map(|pos| (MatchTier::IgnoreCaseSubstring, pos))
}

fn identifier_stem(candidate: &str) -> &str {
    let trimmed = candidate.trim_start();
    let len = trimmed
        .find(|ch: char| !super::is_identifier_char(ch))
        .unwrap_or(trimmed.len());
    if len == 0 {
        trimmed
    } else {
        trimmed.get(..len).unwrap_or(trimmed)
    }
}

fn starts_with_ignore_case(text: &str, prefix: &str) -> bool {
    let mut text_chars = text.chars();
    prefix
        .chars()
        .all(|expected| text_chars.next().is_some_and(|ch| chars_eq(ch, expected)))
}
