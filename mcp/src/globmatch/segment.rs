//! Purpose: Case-insensitive wildcard matching (`*` and `?`) within one path segment.

/// Saved backtrack point for the most recent `*` inside a single segment.
#[derive(Clone, Copy)]
struct WildcardCheckpoint {
    next_pattern_idx: usize,
    retry_text_idx: usize,
}

/// Wildcard match within a single path component: `*` matches any run, `?` one char.
pub(super) fn match_segment(text: &str, pattern: &str) -> bool {
    if text.is_ascii() && pattern.is_ascii() {
        return match_slice(
            text.as_bytes(),
            pattern.as_bytes(),
            |b| b == b'*',
            |t, p| p == b'?' || t.eq_ignore_ascii_case(&p),
        );
    }
    let text: Vec<char> = text.chars().flat_map(char::to_lowercase).collect();
    let pattern: Vec<char> = pattern.chars().flat_map(char::to_lowercase).collect();
    match_slice(&text, &pattern, |c| c == '*', |t, p| p == '?' || t == p)
}

fn match_slice<T: Copy>(
    text: &[T],
    pattern: &[T],
    is_star: impl Fn(T) -> bool,
    matches_one: impl Fn(T, T) -> bool,
) -> bool {
    let (mut text_idx, mut pattern_idx) = (0usize, 0usize);
    let mut checkpoint = None;

    while let Some(&character) = text.get(text_idx) {
        let current = pattern.get(pattern_idx).copied();
        if current.is_some_and(|wanted| matches_one(character, wanted)) {
            text_idx += 1;
            pattern_idx += 1;
        } else if current.is_some_and(&is_star) {
            let next_pattern_idx = pattern_idx + 1;
            checkpoint = Some(WildcardCheckpoint {
                next_pattern_idx,
                retry_text_idx: text_idx + 1,
            });
            pattern_idx = next_pattern_idx;
        } else if let Some(saved) = &mut checkpoint {
            text_idx = saved.retry_text_idx;
            saved.retry_text_idx += 1;
            pattern_idx = saved.next_pattern_idx;
        } else {
            return false;
        }
    }

    pattern.iter().skip(pattern_idx).copied().all(is_star)
}
