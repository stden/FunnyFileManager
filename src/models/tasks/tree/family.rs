//! Purpose: The parent and child links between listed processes, kept only
//! where they can be trusted, and the depth-first walk that lays out each
//! row's tree line.

use std::collections::HashMap;

use super::super::{ProcessRow, ProcessSample, Start};
use super::{Sibling, TreeLine, Visited};

/// Parent and child links between listed rows, by position in the listing.
pub(super) struct Family {
    parents: Vec<Option<usize>>,
    /// Slice bounds into `children` for each parent (`offsets[at]..offsets[at + 1]`).
    offsets: Vec<usize>,
    /// All child positions packed contiguously, grouped by parent in listing order.
    children: Vec<usize>,
}

impl Family {
    /// The links between every row of `rows`.
    pub(super) fn of_all(rows: &[ProcessRow]) -> Self {
        let listed: Vec<&ProcessSample> = rows.iter().map(ProcessRow::sample).collect();
        Self::of(&listed)
    }

    /// Only the parent links between every row of `rows`, without building child lists.
    pub(super) fn parents_of_all(rows: &[ProcessRow]) -> Self {
        let listed: Vec<&ProcessSample> = rows.iter().map(ProcessRow::sample).collect();
        Self {
            parents: trusted_parents(&listed),
            offsets: Vec::new(),
            children: Vec::new(),
        }
    }

    /// The links between the processes `listed`, by position. A PID listed
    /// twice keeps its first position as the parent.
    pub(super) fn of(listed: &[&ProcessSample]) -> Self {
        let parents = trusted_parents(listed);
        let (offsets, children) = build_children(listed.len(), &parents);
        Self {
            parents,
            offsets,
            children,
        }
    }

    /// The trusted parent of position `at`, if it has one.
    pub(super) fn parent(&self, at: usize) -> Option<usize> {
        self.parents.get(at).copied().flatten()
    }

    /// The children of position `at`, in listing order.
    fn children(&self, at: usize) -> &[usize] {
        let start = self.offsets.get(at).copied().unwrap_or(0);
        let end = self.offsets.get(at + 1).copied().unwrap_or(start);
        self.children.get(start..end).unwrap_or_default()
    }

    /// Visits `roots` and everything below them depth first, in order,
    /// calling `visit` with each position and its tree line. Positions
    /// already visited are skipped, so a cycle ends the walk.
    pub(super) fn descend(
        &self,
        roots: &[usize],
        visited: &mut Visited,
        visit: &mut impl FnMut(usize, TreeLine),
    ) {
        let mut stack = Vec::new();
        push_siblings(&mut stack, roots, visited, 0, 0);
        while let Some((at, line)) = stack.pop() {
            visited.insert(at);
            visit(at, line);
            if let TreeLine::Node {
                depth,
                sibling,
                open,
            } = line
            {
                let open = open | continued(depth, sibling);
                push_siblings(
                    &mut stack,
                    self.children(at),
                    visited,
                    depth.saturating_add(1),
                    open,
                );
            }
        }
    }

    /// A member of the cycle that the unvisited `start` hangs from: walking
    /// up from `start` until a position repeats. Every unvisited position
    /// leads into one, since whatever leads to a root was visited from it.
    pub(super) fn cycle_entry(&self, start: usize, visited: &Visited) -> usize {
        let mut walked = Visited::new(self.parents.len());
        let mut at = start;
        while walked.insert(at) {
            match self.parent(at) {
                Some(up) if !visited.contains(up) => at = up,
                _ => break,
            }
        }
        at
    }
}

/// Pushes the unvisited `siblings` so the first pops first, the last marked
/// as such.
fn push_siblings(
    stack: &mut Vec<(usize, TreeLine)>,
    siblings: &[usize],
    visited: &Visited,
    depth: u16,
    open: u64,
) {
    let pending: Vec<usize> = siblings
        .iter()
        .copied()
        .filter(|&at| !visited.contains(at))
        .collect();
    let last = pending.len().saturating_sub(1);
    for (index, &at) in pending.iter().enumerate().rev() {
        let sibling = if index == last {
            Sibling::Last
        } else {
            Sibling::Middle
        };
        let line = TreeLine::Node {
            depth,
            sibling,
            open,
        };
        stack.push((at, line));
    }
}

/// The guide bit a row at `depth` passes to its children: its own connector
/// column continues when a sibling follows it. Roots draw no connector.
fn continued(depth: u16, sibling: Sibling) -> u64 {
    let Some(column) = depth.checked_sub(1) else {
        return 0;
    };
    match sibling {
        Sibling::Middle => 1_u64.checked_shl(u32::from(column)).unwrap_or(0),
        Sibling::Last => 0,
    }
}

/// Pack all child positions contiguously by parent into `(offsets, children)`.
fn build_children(len: usize, parents: &[Option<usize>]) -> (Vec<usize>, Vec<usize>) {
    let mut counts = vec![0usize; len];
    for &up in parents.iter().flatten() {
        if let Some(count) = counts.get_mut(up) {
            *count += 1;
        }
    }
    let offsets: Vec<usize> = std::iter::once(0)
        .chain(counts.into_iter().scan(0, |acc, c| {
            *acc += c;
            Some(*acc)
        }))
        .collect();
    let mut cursor = offsets.clone();
    let mut children = vec![0usize; offsets.last().copied().unwrap_or(0)];
    for (at, &parent) in parents.iter().enumerate() {
        if let Some(slot) = parent.and_then(|up| cursor.get_mut(up)).and_then(|pos| {
            let idx = *pos;
            *pos += 1;
            children.get_mut(idx)
        }) {
            *slot = at;
        }
    }
    (offsets, children)
}

/// Trusted parent position for each entry in `listed`.
fn trusted_parents(listed: &[&ProcessSample]) -> Vec<Option<usize>> {
    let mut by_pid = HashMap::with_capacity(listed.len());
    for (at, sample) in listed.iter().enumerate() {
        by_pid.entry(sample.key.pid()).or_insert(at);
    }
    listed
        .iter()
        .enumerate()
        .map(|(at, child)| {
            let up = *by_pid.get(&child.parent?)?;
            let parent = listed.get(up)?;
            let trusted = up != at && !started_after(parent.key.start(), child.key.start());
            trusted.then_some(up)
        })
        .collect()
}

/// Whether a claimed parent started after its child: then the parent's PID
/// was reused, and the link is stale. Unknown starts cannot tell.
fn started_after(parent: Start, child: Start) -> bool {
    matches!((parent, child), (Start::Known(parent), Start::Known(child)) if parent > child)
}
