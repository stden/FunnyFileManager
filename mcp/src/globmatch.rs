//! One glob matcher for every tool that takes a pattern.
//!
//! `**` spans any number of path components; `*` and `?` stay inside a single
//! component, so `src/*.rs` never matches `src/deep/a.rs`. Matching is
//! case-insensitive, as the per-tool matchers this replaces were.
//!
//! Patterns arrive verbatim from the client, so matching has to stay cheap for
//! hostile input: both levels backtrack iteratively (O(text × pattern), constant
//! stack) rather than recursing, `**` runs are collapsed, and an absurdly large
//! pattern is refused outright.

#[cfg(test)]
mod tests;

/// Upper bound on the components of a pattern. Real trees are nowhere near this
/// deep, so refusing more costs nothing — and it caps the work a single client
/// string can impose on every entry of a scanned tree.
pub const MAX_PATTERN_COMPONENTS: usize = 64;

/// Upper bound on a pattern in bytes, for the same reason: a single component
/// may be arbitrarily long, and each one is matched against every candidate.
pub const MAX_PATTERN_LEN: usize = 4096;

/// Match a `/`-separated candidate against a glob pattern.
/// A bare file name is simply a one-component path.
///
/// An oversized pattern never matches: this is the fail-closed half of
/// [`validate_pattern`], so a caller that skipped validation cannot turn a
/// hostile string into unbounded scanning work.
pub fn matches(candidate: &str, pattern: &str) -> bool {
    if validate_pattern(pattern).is_err() {
        return false;
    }
    match_components(&components(candidate), &pattern_components(pattern))
}

/// Reject a pattern too large to be meaningful, so a tool can report a clean
/// error instead of silently matching nothing.
pub fn validate_pattern(pattern: &str) -> Result<(), String> {
    if pattern.len() > MAX_PATTERN_LEN {
        return Err(format!(
            "glob pattern is {} bytes, the limit is {MAX_PATTERN_LEN}",
            pattern.len()
        ));
    }
    let count = pattern_components(pattern).len();
    if count > MAX_PATTERN_COMPONENTS {
        return Err(format!(
            "glob pattern has {count} components, the limit is {MAX_PATTERN_COMPONENTS}"
        ));
    }
    Ok(())
}

/// How deep below the search root a pattern can still match, or `None` when
/// `**` makes it unbounded. Lets callers stop descending early.
pub fn max_depth(pattern: &str) -> Option<usize> {
    let parts = pattern_components(pattern);
    if parts.contains(&"**") {
        return None;
    }
    Some(parts.len().saturating_sub(1))
}

fn components(path: &str) -> Vec<&str> {
    path.split('/')
        .filter(|part| !part.is_empty() && *part != ".")
        .collect()
}

/// Pattern components with consecutive `**` runs collapsed.
///
/// `**/**` matches exactly what `**` matches, so folding the run away is
/// semantics-preserving — and it is what stops `"**/"` repeated N times from
/// handing the matcher N independent backtrack points.
fn pattern_components(pattern: &str) -> Vec<&str> {
    let mut parts = components(pattern);
    parts.dedup_by(|current, previous| *current == "**" && *previous == "**");
    parts
}

/// Match component-by-component, where `**` consumes zero or more components.
///
/// The same greedy scan with a single backtrack point as [`match_segment`], one
/// level up. It replaces a double recursion that had no memoization: each `**`
/// multiplied the search space, so a few dozen bytes of pattern took seconds and
/// a few kilobytes overflowed the stack.
fn match_components(candidate: &[&str], pattern: &[&str]) -> bool {
    let (mut ci, mut pi) = (0usize, 0usize);
    let mut star: Option<usize> = None;
    let mut star_ci = 0usize;

    while let Some(&part) = candidate.get(ci) {
        let current = pattern.get(pi).copied();
        if current == Some("**") {
            star = Some(pi);
            star_ci = ci;
            pi += 1;
        } else if current.is_some_and(|segment| match_segment(part, segment)) {
            ci += 1;
            pi += 1;
        } else if let Some(star_pi) = star {
            // Let the last `**` swallow one more component and retry.
            star_ci += 1;
            ci = star_ci;
            pi = star_pi + 1;
        } else {
            return false;
        }
    }

    pattern.iter().skip(pi).all(|part| *part == "**")
}

/// Wildcard match within a single path component: `*` matches any run of
/// characters, `?` exactly one. Backtracks on the last `*` seen.
fn match_segment(text: &str, pattern: &str) -> bool {
    let text: Vec<char> = text.to_lowercase().chars().collect();
    let pattern: Vec<char> = pattern.to_lowercase().chars().collect();

    let (mut ti, mut pi) = (0usize, 0usize);
    let mut star: Option<usize> = None;
    let mut star_ti = 0usize;

    while let Some(&character) = text.get(ti) {
        let current = pattern.get(pi).copied();
        if current.is_some_and(|wanted| wanted == '?' || wanted == character) {
            ti += 1;
            pi += 1;
        } else if current == Some('*') {
            star = Some(pi);
            star_ti = ti;
            pi += 1;
        } else if let Some(star_pi) = star {
            star_ti += 1;
            ti = star_ti;
            pi = star_pi + 1;
        } else {
            return false;
        }
    }

    pattern.iter().skip(pi).all(|c| *c == '*')
}
