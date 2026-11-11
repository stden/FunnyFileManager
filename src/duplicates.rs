//! Purpose: finds the blocks of source text that repeat across a set of texts.
//!
//! Pure: no file, no `App`. The folder walk that supplies the texts is in
//! `ops::duplicates`. Blank lines do not count and inner whitespace is
//! collapsed, so reindenting or respacing a copy hides nothing.

use std::cmp::Ordering;
use std::time::Instant;

mod groups;
mod lines;
mod windows;

/// One place a block occurs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Occurrence {
    /// The index of the text in the list given to [`find`].
    pub source: usize,
    /// The 1-based line of the block's first non-blank line.
    pub first_line: usize,
    /// The 1-based line of its last non-blank line.
    pub last_line: usize,
}

/// A block of `block_lines` non-blank lines and every place it occurs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub block_lines: usize,
    pub occurrences: Vec<Occurrence>,
}

impl Group {
    /// The lines the copies after the first add up to.
    #[must_use]
    pub fn excess_lines(&self) -> usize {
        self.occurrences
            .len()
            .saturating_sub(1)
            .saturating_mul(self.block_lines)
    }
}

/// What a search found, and whether it compared everything before its deadline.
pub struct Search {
    pub groups: Vec<Group>,
    pub complete: bool,
}

/// The `top` groups that repeat the most, most repeated first. Stops comparing
/// at `deadline`, and then says the list is incomplete.
///
/// `texts` must be given in path order: a tie is broken by the index of the
/// text, so the order they arrive in decides it.
pub fn find(texts: &[&str], min_block: usize, top: usize, deadline: Instant) -> Search {
    if min_block == 0 {
        return Search {
            groups: Vec::new(),
            complete: true,
        };
    }
    let (mut found, complete) = groups::repeated_blocks(texts, min_block, deadline);
    found.sort_by(rank);
    found.truncate(top);
    Search {
        groups: found,
        complete,
    }
}

/// Most excess lines first, then longer blocks, then the earlier place.
fn rank(left: &Group, right: &Group) -> Ordering {
    right
        .excess_lines()
        .cmp(&left.excess_lines())
        .then(right.block_lines.cmp(&left.block_lines))
        .then_with(|| left.occurrences.cmp(&right.occurrences))
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_deadline;
