//! The iterative wildcard matcher shared by ASCII and Unicode file masks.

/// Match one pattern slice against one candidate slice with iterative
/// single-star backtracking.
pub(super) fn glob_slice<T: Copy>(
    pattern: &[T],
    name: &[T],
    is_star: impl Fn(T) -> bool,
    matches_one: impl Fn(T, T) -> bool,
) -> bool {
    let (mut pat_idx, mut name_idx) = (0, 0);
    let mut last_star_idx: Option<usize> = None;
    let mut star_match_end = 0;

    while let Some(&current) = name.get(name_idx) {
        match pattern.get(pat_idx).copied() {
            Some(ch) if is_star(ch) => {
                last_star_idx = Some(pat_idx);
                star_match_end = name_idx;
                pat_idx += 1;
            }
            Some(ch) if matches_one(ch, current) => {
                pat_idx += 1;
                name_idx += 1;
            }
            _ => {
                let Some(star_idx) = last_star_idx else {
                    return false;
                };
                pat_idx = star_idx + 1;
                star_match_end += 1;
                name_idx = star_match_end;
            }
        }
    }
    let rest = pattern.get(pat_idx..).unwrap_or_default();
    rest.iter().copied().all(is_star)
}
