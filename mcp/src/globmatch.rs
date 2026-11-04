//! Purpose: One glob matcher for every tool that takes a pattern.
//!
//! `**` spans any number of path components; `*` and `?` stay inside a single
//! component, so `src/*.rs` never matches `src/deep/a.rs`. Matching is
//! case-insensitive, as the per-tool matchers this replaces were.

mod segment;
#[cfg(test)]
mod tests;

use segment::match_segment;

/// Upper bound on the components of a pattern.
pub const MAX_PATTERN_COMPONENTS: usize = 64;

/// Upper bound on a pattern in bytes.
pub const MAX_PATTERN_LEN: usize = 4096;

/// Pre-validated, collapsed glob pattern reused across an entire directory walk.
pub struct CompiledGlob<'a> {
    parts: Vec<&'a str>,
    first_double_star: Option<usize>,
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
        let first_double_star = parts.iter().position(|&part| part == "**");
        Ok(Self {
            parts,
            first_double_star,
        })
    }

    /// How deep below the search root this pattern can match, or `None` when `**` makes it unbounded.
    pub fn max_depth(&self) -> Option<usize> {
        self.first_double_star
            .is_none()
            .then(|| self.parts.len().saturating_sub(1))
    }

    /// Whether a directory named `segment` at `depth` (0-based from root) can
    /// contain any matching descendants, assuming ancestors `0..depth` matched.
    pub fn can_descend(&self, depth: usize, segment: &str) -> bool {
        if self
            .first_double_star
            .is_some_and(|star_idx| depth >= star_idx)
        {
            return true;
        }
        if self.max_depth().is_some_and(|max| depth >= max) {
            return false;
        }
        self.parts
            .get(depth)
            .is_some_and(|wanted| match_segment(segment, wanted))
    }

    pub fn matches(&self, candidate: &str) -> bool {
        if let [single_part] = self.parts.as_slice() {
            if is_single_component(candidate) {
                return *single_part == "**" || match_segment(candidate, single_part);
            }
        }
        match_components(candidate, &self.parts)
    }
}

fn is_single_component(candidate: &str) -> bool {
    !candidate.is_empty() && candidate != "." && !candidate.contains('/')
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

/// Pattern components with consecutive `**` runs collapsed.
fn pattern_components(pattern: &str) -> Vec<&str> {
    let mut parts: Vec<&str> = pattern
        .split('/')
        .filter(|part| !part.is_empty() && *part != ".")
        .collect();
    parts.dedup_by(|current, previous| *current == "**" && *previous == "**");
    parts
}

fn next_component(mut rest: &str) -> Option<(&str, &str)> {
    loop {
        let (part, tail) = rest.split_once('/').unwrap_or((rest, ""));
        rest = tail;
        if !part.is_empty() && part != "." {
            return Some((part, rest));
        }
        if rest.is_empty() {
            return None;
        }
    }
}

/// Saved backtrack point for the most recent `**` component across path segments.
#[derive(Clone, Copy)]
struct DoubleStarCheckpoint<'a> {
    pattern_idx: usize,
    remaining_path: &'a str,
}

/// Match component-by-component without allocating a `Vec<&str>` per candidate.
fn match_components(mut rest: &str, pattern: &[&str]) -> bool {
    let mut pattern_idx = 0usize;
    let mut checkpoint = None;

    while let Some((part, after_part)) = next_component(rest) {
        let current = pattern.get(pattern_idx).copied();
        if current == Some("**") {
            checkpoint = Some(DoubleStarCheckpoint {
                pattern_idx,
                remaining_path: after_part,
            });
            pattern_idx += 1;
        } else if current.is_some_and(|segment| match_segment(part, segment)) {
            rest = after_part;
            pattern_idx += 1;
        } else if let Some(saved) = checkpoint.take() {
            rest = saved.remaining_path;
            pattern_idx = saved.pattern_idx;
        } else {
            return false;
        }
    }

    pattern.iter().skip(pattern_idx).all(|part| *part == "**")
}
