//! One glob matcher for every tool that takes a pattern.
//!
//! `**` spans any number of path components; `*` and `?` stay inside a single
//! component, so `src/*.rs` never matches `src/deep/a.rs`. Matching is
//! case-insensitive, as the per-tool matchers this replaces were.

#[cfg(test)]
mod tests;

/// Upper bound on the components of a pattern.
pub const MAX_PATTERN_COMPONENTS: usize = 64;

/// Upper bound on a pattern in bytes.
pub const MAX_PATTERN_LEN: usize = 4096;

/// Pre-validated, collapsed glob pattern reused across an entire directory walk.
pub struct CompiledGlob<'a> {
    parts: Vec<&'a str>,
    has_double_star: bool,
}

impl<'a> CompiledGlob<'a> {
    pub fn compile(pattern: &'a str) -> Result<Self, String> {
        if pattern.len() > MAX_PATTERN_LEN {
            return Err(format!(
                "glob pattern is {} bytes, the limit is {MAX_PATTERN_LEN}",
                pattern.len()
            ));
        }
        let parts = pattern_components(pattern);
        if parts.len() > MAX_PATTERN_COMPONENTS {
            return Err(format!(
                "glob pattern has {} components, the limit is {MAX_PATTERN_COMPONENTS}",
                parts.len()
            ));
        }
        let has_double_star = parts.contains(&"**");
        Ok(Self {
            parts,
            has_double_star,
        })
    }

    /// How deep below the search root this pattern can match, or `None` when `**` makes it unbounded.
    pub fn max_depth(&self) -> Option<usize> {
        (!self.has_double_star).then(|| self.parts.len().saturating_sub(1))
    }

    pub fn matches(&self, candidate: &str) -> bool {
        if !candidate.contains('/') && !candidate.is_empty() && candidate != "." {
            if self.parts.len() == 1 {
                return self
                    .parts
                    .first()
                    .is_some_and(|seg| *seg == "**" || match_segment(candidate, seg));
            }
            return match_components(&[candidate], &self.parts);
        }
        match_components(&components(candidate), &self.parts)
    }
}

/// Match a `/`-separated candidate against a glob pattern.
#[cfg(test)]
pub fn matches(candidate: &str, pattern: &str) -> bool {
    CompiledGlob::compile(pattern).is_ok_and(|glob| glob.matches(candidate))
}

/// Reject a pattern too large to be meaningful.
#[cfg(test)]
pub fn validate_pattern(pattern: &str) -> Result<(), String> {
    CompiledGlob::compile(pattern).map(|_| ())
}

/// How deep below the search root a pattern can still match, or `None` when `**` makes it unbounded.
#[cfg(test)]
pub fn max_depth(pattern: &str) -> Option<usize> {
    let parts = pattern_components(pattern);
    (!parts.contains(&"**")).then(|| parts.len().saturating_sub(1))
}

fn components(path: &str) -> Vec<&str> {
    path.split('/')
        .filter(|part| !part.is_empty() && *part != ".")
        .collect()
}

/// Pattern components with consecutive `**` runs collapsed.
fn pattern_components(pattern: &str) -> Vec<&str> {
    let mut parts = components(pattern);
    parts.dedup_by(|current, previous| *current == "**" && *previous == "**");
    parts
}

/// Match component-by-component, where `**` consumes zero or more components.
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
            star_ci += 1;
            ci = star_ci;
            pi = star_pi + 1;
        } else {
            return false;
        }
    }

    pattern.iter().skip(pi).all(|part| *part == "**")
}

/// Wildcard match within a single path component: `*` matches any run, `?` one char.
fn match_segment(text: &str, pattern: &str) -> bool {
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
    let (mut ti, mut pi) = (0usize, 0usize);
    let mut star: Option<usize> = None;
    let mut star_ti = 0usize;

    while let Some(&character) = text.get(ti) {
        let current = pattern.get(pi).copied();
        if current.is_some_and(|wanted| matches_one(character, wanted)) {
            ti += 1;
            pi += 1;
        } else if current.is_some_and(&is_star) {
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

    pattern
        .get(pi..)
        .unwrap_or_default()
        .iter()
        .copied()
        .all(is_star)
}
