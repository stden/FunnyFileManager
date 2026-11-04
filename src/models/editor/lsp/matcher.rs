//! Purpose: `IntelliJ` IDEA-style completion matching and ranking (exact prefix,
//! case-insensitive prefix, `CamelHumps` / `snake_case` word boundaries, and substring).

use crate::lsp::features::Completion;

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

pub fn filter_and_rank(items: &[Completion], prefix: &str) -> Vec<Completion> {
    if prefix.is_empty() {
        return items.to_vec();
    }
    let mut ranks: Vec<MatchRank> = items
        .iter()
        .enumerate()
        .filter_map(|(original_index, item)| {
            let (tier, penalty) =
                score(&item.label, prefix).or_else(|| score(&item.insert_text, prefix))?;
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

fn score(candidate: &str, prefix: &str) -> Option<(MatchTier, usize)> {
    let stem = identifier_stem(candidate);
    if stem.is_empty() {
        return None;
    }
    if stem.starts_with(prefix) {
        return Some((MatchTier::ExactCasePrefix, usize::from(stem != prefix)));
    }
    if starts_with_ignore_case(stem, prefix) {
        let exact_len = stem.chars().count() == prefix.chars().count();
        return Some((MatchTier::IgnoreCasePrefix, usize::from(!exact_len)));
    }
    if let Some(skipped) = camel_humps(stem, prefix) {
        return Some((MatchTier::CamelHumps, skipped));
    }
    if let Some(pos) = stem.find(prefix) {
        return Some((MatchTier::ExactCaseSubstring, pos));
    }
    find_ignore_case(stem, prefix).map(|pos| (MatchTier::IgnoreCaseSubstring, pos))
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

fn find_ignore_case(text: &str, needle: &str) -> Option<usize> {
    crate::casefold::find_ignore_case(text, needle)
}

fn camel_humps(stem: &str, prefix: &str) -> Option<usize> {
    let words = split_words(stem);
    let query: Vec<char> = prefix.chars().filter(|&ch| ch != '_').collect();
    if words.len() < 2 || query.len() < 2 {
        return None;
    }
    (0..words.len())
        .filter_map(|start| {
            let tail = words.get(start..)?;
            match_humps(tail, &query, false).map(|skipped| start + skipped)
        })
        .min()
}

fn split_words(stem: &str) -> Vec<Vec<char>> {
    let mut words: Vec<Vec<char>> = Vec::new();
    let mut current: Vec<char> = Vec::new();
    for ch in stem.chars() {
        if ch == '_' {
            if !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
            continue;
        }
        let boundary = ch.is_uppercase()
            && current
                .last()
                .is_some_and(|prev| prev.is_lowercase() || prev.is_ascii_digit());
        if boundary && !current.is_empty() {
            words.push(std::mem::take(&mut current));
        }
        current.push(ch);
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

fn match_humps(words: &[Vec<char>], query: &[char], used_multiple_words: bool) -> Option<usize> {
    let first_word = words.first()?;
    let max_take = first_word
        .iter()
        .zip(query.iter())
        .take_while(|(&a, &b)| chars_eq(a, b))
        .count();
    for take in (1..=max_take).rev() {
        if take == query.len() {
            if used_multiple_words {
                return Some(0);
            }
            continue;
        }
        let rem_query = query.get(take..).unwrap_or_default();
        if let Some(skipped) = match_remaining_words(words, rem_query) {
            return Some(skipped);
        }
    }
    None
}

fn match_remaining_words(words: &[Vec<char>], rem_query: &[char]) -> Option<usize> {
    for next_idx in 1..words.len() {
        let tail_words = words.get(next_idx..).unwrap_or_default();
        if let Some(skipped) = match_humps(tail_words, rem_query, true) {
            return Some((next_idx - 1) + skipped);
        }
    }
    None
}

fn chars_eq(a: char, b: char) -> bool {
    a.eq_ignore_ascii_case(&b) || a.to_lowercase().eq(b.to_lowercase())
}
