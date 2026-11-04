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

/// One subsequence walk: the points, the char index of the last hit, and how
/// many query characters matched. A hit on `0..matched_count` is a prefix.
struct Hit {
    points: i32,
    last_match_idx: usize,
    matched_count: usize,
}

impl Hit {
    /// Every matched character landed at `0..matched_count` without any leading or internal gap.
    const fn is_prefix(&self) -> bool {
        self.last_match_idx + 1 == self.matched_count
    }
}

/// Precomputed query metadata hoisted outside candidate loops.
pub(super) struct PreparedQuery<'a> {
    raw: &'a str,
    case_sensitive: bool,
    ascii: bool,
}

impl<'a> PreparedQuery<'a> {
    pub(super) fn new(query: &'a str) -> Self {
        let ascii = query.is_ascii();
        let case_sensitive = if ascii {
            query.bytes().any(|b| b.is_ascii_uppercase())
        } else {
            query.chars().any(char::is_uppercase)
        };
        Self {
            raw: query,
            case_sensitive,
            ascii,
        }
    }

    pub(super) fn score(&self, candidate: &str) -> Option<Score> {
        if self.raw.is_empty() {
            return Some(Score {
                tier: Tier::Prefix,
                points: 0,
            });
        }
        let query_bytes = self.raw.as_bytes();
        if self.ascii
            && !has_ascii_subsequence(candidate.as_bytes(), query_bytes, self.case_sensitive)
        {
            return None;
        }
        let hit = walk(candidate, self.raw, self.case_sensitive)?;
        let tier = if hit.is_prefix() {
            Tier::Prefix
        } else {
            Tier::Fuzzy
        };
        Some(Score {
            tier,
            points: hit.points,
        })
    }
}

fn has_ascii_subsequence(mut haystack: &[u8], query: &[u8], case_sensitive: bool) -> bool {
    if haystack.len() < query.len() {
        return false;
    }
    for &wanted in query {
        let idx = if !case_sensitive && wanted.is_ascii_alphabetic() {
            memchr::memchr2(wanted, wanted.to_ascii_uppercase(), haystack)
        } else {
            memchr::memchr(wanted, haystack)
        };
        let Some(at) = idx else { return false };
        haystack = haystack.get(at + 1..).unwrap_or_default();
    }
    true
}

/// Score `candidate` against `query`, or `None` when it does not match.
#[must_use]
pub fn score(query: &str, candidate: &str) -> Option<Score> {
    PreparedQuery::new(query).score(candidate)
}

#[derive(Default)]
struct WalkState {
    points: i32,
    prev_match_idx: Option<usize>,
    gap_start_idx: usize,
    matched_count: usize,
    prev_char: Option<char>,
}

fn walk(candidate: &str, query: &str, case_sensitive: bool) -> Option<Hit> {
    let mut wanted = query.chars().map(|ch| fold(ch, case_sensitive));
    let mut need = wanted.next()?;
    let mut state = WalkState::default();
    for (index, ch) in candidate.chars().enumerate() {
        let ch = fold(ch, case_sensitive);
        if ch == need {
            let hit = record(&mut state, index);
            let Some(next) = wanted.next() else {
                return Some(hit);
            };
            need = next;
        }
        state.prev_char = Some(ch);
    }
    None
}

fn record(state: &mut WalkState, index: usize) -> Hit {
    state.points += bonus(state.prev_match_idx, index, state.prev_char);
    state.points -= GAP_PENALTY * i32::try_from(index - state.gap_start_idx).unwrap_or(0);
    state.prev_match_idx = Some(index);
    state.gap_start_idx = index + 1;
    state.matched_count += 1;
    Hit {
        points: state.points,
        last_match_idx: index,
        matched_count: state.matched_count,
    }
}

fn fold(ch: char, case_sensitive: bool) -> char {
    if case_sensitive {
        ch
    } else {
        ch.to_ascii_lowercase()
    }
}

fn bonus(prev_match_idx: Option<usize>, index: usize, prev_char: Option<char>) -> i32 {
    if prev_match_idx.is_some_and(|prev| prev + 1 == index) {
        CONSECUTIVE_BONUS
    } else if index == 0 || prev_char.is_some_and(|ch| WORD_BOUNDARIES.contains(&ch)) {
        BOUNDARY_BONUS
    } else {
        PLAIN_MATCH
    }
}

/// Deterministic comparison: `(tier desc, points desc, length asc, lexicographic asc)`.
pub(super) fn cmp_ranked(a_score: Score, a: &str, b_score: Score, b: &str) -> std::cmp::Ordering {
    b_score
        .tier
        .cmp(&a_score.tier)
        .then(b_score.points.cmp(&a_score.points))
        .then(a.len().cmp(&b.len()))
        .then_with(|| a.cmp(b))
}

/// Keep the candidates matching `query`, best first.
#[must_use]
pub fn rank(query: &str, candidates: Vec<String>) -> Vec<String> {
    let prepared = PreparedQuery::new(query);
    let mut scored: Vec<(Score, String)> = candidates
        .into_iter()
        .filter_map(|candidate| prepared.score(&candidate).map(|s| (s, candidate)))
        .collect();
    scored.sort_by(|(a_score, a), (b_score, b)| cmp_ranked(*a_score, a, *b_score, b));
    scored.into_iter().map(|(_, candidate)| candidate).collect()
}

#[cfg(test)]
mod tests;
