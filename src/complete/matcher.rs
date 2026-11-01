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
/// many query characters matched. A hit on `0..matched` is a prefix.
struct Hit {
    points: i32,
    last: usize,
    matched: usize,
}

/// Score `candidate` against `query`, or `None` when it does not match.
///
/// Matching is subsequence-based: every query character must appear in
/// `candidate`, in order. **Smartcase** — an all-lowercase query matches
/// case-insensitively; any uppercase character makes the query case-sensitive.
///
/// The walk is one pass and allocates nothing. Smartcase folds each character
/// as it is read, with `to_ascii_lowercase`, which is what the previous
/// lowered copies compared.
#[must_use]
pub fn score(query: &str, candidate: &str) -> Option<Score> {
    if query.is_empty() {
        return Some(Score {
            tier: Tier::Prefix,
            points: 0,
        });
    }
    let case_sensitive = query.chars().any(char::is_uppercase);
    let hit = walk(candidate, query, case_sensitive)?;
    let tier = if hit.last + 1 == hit.matched {
        Tier::Prefix
    } else {
        Tier::Fuzzy
    };
    Some(Score {
        tier,
        points: hit.points,
    })
}

fn walk(candidate: &str, query: &str, case_sensitive: bool) -> Option<Hit> {
    let mut wanted = query.chars().map(|ch| fold(ch, case_sensitive));
    let mut need = wanted.next()?;
    let mut points = 0;
    let mut previous = None;
    let mut from = 0;
    let mut matched = 0;
    let mut before = None;
    for (index, ch) in candidate.chars().enumerate() {
        let ch = fold(ch, case_sensitive);
        if ch == need {
            let hit = record(
                &mut points,
                &mut previous,
                &mut from,
                &mut matched,
                index,
                before,
            );
            need = match wanted.next() {
                Some(next) => next,
                None => return Some(hit),
            };
        }
        before = Some(ch);
    }
    None
}

fn record(
    points: &mut i32,
    previous: &mut Option<usize>,
    from: &mut usize,
    matched: &mut usize,
    index: usize,
    before: Option<char>,
) -> Hit {
    *points += bonus(*previous, index, before);
    *points -= GAP_PENALTY * i32::try_from(index - *from).unwrap_or(0);
    *previous = Some(index);
    *from = index + 1;
    *matched += 1;
    Hit {
        points: *points,
        last: index,
        matched: *matched,
    }
}

fn fold(ch: char, case_sensitive: bool) -> char {
    if case_sensitive {
        ch
    } else {
        ch.to_ascii_lowercase()
    }
}

fn bonus(previous: Option<usize>, index: usize, before: Option<char>) -> i32 {
    if previous == Some(index.wrapping_sub(1)) {
        CONSECUTIVE_BONUS
    } else if index == 0 || before.is_some_and(|ch| WORD_BOUNDARIES.contains(&ch)) {
        BOUNDARY_BONUS
    } else {
        PLAIN_MATCH
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
