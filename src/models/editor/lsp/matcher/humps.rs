//! Purpose: Zero-allocation `O(W * Q)` dynamic programming for `CamelHumps` / `snake_case`
//! multi-word completion matching.

#[derive(Default)]
pub(super) struct Scratch {
    chars: Vec<char>,
    words: Vec<(usize, usize)>,
    dp: Vec<usize>,
}

pub(super) fn camel_humps(stem: &str, query: &[char], scratch: &mut Scratch) -> Option<usize> {
    let q_len = query.len();
    if q_len < 2 {
        return None;
    }
    split_words_into(stem, &mut scratch.chars, &mut scratch.words);
    if scratch.words.len() < 2 {
        return None;
    }
    scratch.dp.clear();
    scratch.dp.resize(q_len, 0);
    let mut best: Option<usize> = None;
    for (word_idx, &(lo, hi)) in scratch.words.iter().enumerate() {
        let word = scratch.chars.get(lo..hi).unwrap_or_default();
        if let Some(skipped) = advance_dp(&mut scratch.dp, word_idx, word, query) {
            best = Some(best.map_or(skipped, |cur| cur.min(skipped)));
        }
    }
    best
}

pub(super) fn chars_eq(a: char, b: char) -> bool {
    a.eq_ignore_ascii_case(&b) || a.to_lowercase().eq(b.to_lowercase())
}

fn split_words_into(stem: &str, chars: &mut Vec<char>, words: &mut Vec<(usize, usize)>) {
    chars.clear();
    words.clear();
    let mut start = 0;
    for ch in stem.chars() {
        if ch == '_' {
            if chars.len() > start {
                words.push((start, chars.len()));
                start = chars.len();
            }
            continue;
        }
        let boundary = ch.is_uppercase()
            && chars.len() > start
            && chars
                .last()
                .is_some_and(|prev| prev.is_lowercase() || prev.is_ascii_digit());
        if boundary {
            words.push((start, chars.len()));
            start = chars.len();
        }
        chars.push(ch);
    }
    if chars.len() > start {
        words.push((start, chars.len()));
    }
}

fn advance_dp(dp: &mut [usize], word_idx: usize, word: &[char], query: &[char]) -> Option<usize> {
    let q_len = query.len();
    let mut word_best: Option<usize> = None;
    for j in (0..q_len).rev() {
        let prev_words = dp.get(j).copied().unwrap_or(0);
        if j > 0 && prev_words == 0 {
            continue;
        }
        let lcp = common_prefix_len(word, query.get(j..).unwrap_or_default());
        if prev_words > 0 && j + lcp == q_len {
            let skipped = word_idx.saturating_sub(prev_words);
            word_best = Some(word_best.map_or(skipped, |cur| cur.min(skipped)));
        }
        let max_end = (j + lcp).min(q_len - 1);
        for end in (j + 1)..=max_end {
            if let Some(slot) = dp.get_mut(end) {
                *slot = (*slot).max(prev_words + 1);
            }
        }
    }
    word_best
}

fn common_prefix_len(word: &[char], query: &[char]) -> usize {
    word.iter()
        .zip(query.iter())
        .take_while(|(&a, &b)| chars_eq(a, b))
        .count()
}
