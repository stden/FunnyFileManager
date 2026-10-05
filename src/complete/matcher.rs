//! Purpose: Fuzzy matching and deterministic ranking of completion candidates.
//!
//! Pure: no `App`, no filesystem, no I/O — everything here is a function of its
//! arguments, which is what makes the ranking cheap to test exhaustively.

/// Ranking tier. Exact-prefix matches always beat fuzzy ones, so typing `cd`
/// can never be outranked by a fuzzy hit on some longer verb.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    Fuzzy,
    Prefix,
}

/// How well a candidate matches a query; higher `points` is better.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Score {
    pub tier: Tier,
    pub points: i32,
}

/// Characters after which a match counts as starting a new word.
const WORD_BOUNDARIES: [char; 5] = ['/', '_', '-', '.', ' '];

/// Points awarded for a match continuing a run, opening a word, or neither.
const CONSECUTIVE_BONUS: i32 = 10;
const BOUNDARY_BONUS: i32 = 8;
const PLAIN_MATCH: i32 = 1;
/// Deducted per candidate character skipped over.
const GAP_PENALTY: i32 = 1;

/// Score `candidate` against `query`, or `None` when it does not match.
///
/// Matching is subsequence-based: every query character must appear in
/// `candidate`, in order. **Smartcase** — an all-lowercase query matches
/// case-insensitively; any uppercase character makes the query case-sensitive.
#[must_use]
pub fn score(query: &str, candidate: &str) -> Option<Score> {
    if query.is_empty() {
        return Some(Score {
            tier: Tier::Prefix,
            points: 0,
        });
    }

    let case_sensitive = query.chars().any(char::is_uppercase);
    let normalize = |c: char| {
        if case_sensitive {
            c
        } else {
            c.to_ascii_lowercase()
        }
    };

    let haystack: Vec<char> = candidate.chars().map(normalize).collect();
    let needle: Vec<char> = query.chars().map(normalize).collect();

    let points = subsequence_points(&haystack, &needle)?;

    let tier = if is_prefix(&needle_of(query, case_sensitive), candidate, case_sensitive) {
        Tier::Prefix
    } else {
        Tier::Fuzzy
    };

    Some(Score { tier, points })
}

fn subsequence_points(haystack: &[char], needle: &[char]) -> Option<i32> {
    let mut points = 0;
    let mut haystack_index = 0;
    let mut previous_match: Option<usize> = None;

    for &wanted in needle {
        let found = haystack
            .get(haystack_index..)?
            .iter()
            .position(|&c| c == wanted)
            .map(|offset| haystack_index + offset)?;

        points += if previous_match == Some(found.wrapping_sub(1)) {
            CONSECUTIVE_BONUS
        } else if found == 0
            || haystack
                .get(found - 1)
                .is_some_and(|c| WORD_BOUNDARIES.contains(c))
        {
            BOUNDARY_BONUS
        } else {
            PLAIN_MATCH
        };
        points -= GAP_PENALTY * i32::try_from(found - haystack_index).unwrap_or(0);

        previous_match = Some(found);
        haystack_index = found + 1;
    }

    Some(points)
}

/// The query as compared — lowercased unless the query is case-sensitive.
fn needle_of(query: &str, case_sensitive: bool) -> String {
    if case_sensitive {
        query.to_owned()
    } else {
        query.to_ascii_lowercase()
    }
}

/// Whether `candidate` starts with `needle` under the active case rule.
fn is_prefix(needle: &str, candidate: &str, case_sensitive: bool) -> bool {
    if case_sensitive {
        candidate.starts_with(needle)
    } else {
        candidate.to_ascii_lowercase().starts_with(needle)
    }
}

/// Keep the candidates matching `query`, best first.
///
/// The ordering key is `(tier desc, points desc, length asc, lexicographic asc)`
/// — total and deterministic, because a ranking that varies between runs cannot
/// be tested.
#[must_use]
pub fn rank(query: &str, candidates: Vec<String>) -> Vec<String> {
    let mut scored: Vec<(Score, String)> = candidates
        .into_iter()
        .filter_map(|candidate| score(query, &candidate).map(|s| (s, candidate)))
        .collect();

    scored.sort_by(|(a_score, a), (b_score, b)| {
        b_score
            .tier
            .cmp(&a_score.tier)
            .then(b_score.points.cmp(&a_score.points))
            .then(a.len().cmp(&b.len()))
            .then(a.cmp(b))
    });

    scored.into_iter().map(|(_, candidate)| candidate).collect()
}

#[cfg(test)]
mod tests;
