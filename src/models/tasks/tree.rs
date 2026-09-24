//! Purpose: The parent tree of a process table — who is whose child, which
//! processes descend from one or lead to one, and the guides each tree row
//! draws — built without recursion, over parent links that cannot be trusted.
//!
//! A parent PID comes from a file the kernel writes but the source only
//! reads: by the time it is read the parent may have exited and its PID gone
//! to a newcomer, and a hostile or broken source can report a process as its
//! own parent or two processes as each other's. So a link is kept only when
//! the parent is listed, is not the child itself, and did not start after it;
//! every walk keeps a visited set; and nothing recurses, so a chain ten
//! thousand deep costs a loop, not the stack.

use std::collections::HashMap;

use super::view::ViewRow;
use super::{Pid, ProcessRow, ProcessSample, Start};

/// Where a row sits in the tree view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeLine {
    /// The flat list: no guides.
    Flat,
    Node {
        /// 0 for a root; a root draws no connector.
        depth: u16,
        /// Whether a sibling follows under the same parent: `├─` or `└─`.
        sibling: Sibling,
        /// Bit `k` is set when the ancestor at depth `k + 1` has a sibling
        /// still to come, so guide column `k` continues with `│`. Columns
        /// from 64 on have no bit and are drawn blank.
        open: u64,
    },
}

impl TreeLine {
    /// Whether guide column `column` of this row continues a line from above.
    #[must_use]
    pub fn continues(self, column: u16) -> bool {
        match self {
            Self::Flat => false,
            Self::Node { open, .. } => 1_u64
                .checked_shl(u32::from(column))
                .is_some_and(|bit| open & bit != 0),
        }
    }
}

/// Whether a tree row has a sibling below it under the same parent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sibling {
    Middle,
    Last,
}

/// The shown rows `shown` (each with its index in the snapshot, already
/// sorted) as the parent tree: each process under its parent, siblings in the
/// order given. A process whose parent is not shown is a root; a cycle is
/// entered once, at the member reached first.
pub(super) fn arrange(shown: &[(usize, &ProcessRow)]) -> Vec<ViewRow> {
    let samples: Vec<&ProcessSample> = shown.iter().map(|(_, row)| row.sample()).collect();
    let family = Family::of(&samples);
    let mut arranged = Vec::with_capacity(shown.len());
    let mut visited = Visited::new(shown.len());
    let roots: Vec<usize> = (0..shown.len())
        .filter(|&at| family.parent(at).is_none())
        .collect();
    let mut place = |at: usize, line| {
        if let Some(&(row, process)) = shown.get(at) {
            arranged.push(ViewRow::new(row, process, line));
        }
    };
    family.descend(&roots, &mut visited, &mut place);
    for start in 0..shown.len() {
        if !visited.contains(start) {
            let entry = family.cycle_entry(start, &visited);
            family.descend(&[entry], &mut visited, &mut place);
        }
    }
    arranged
}

/// The row `root` of `rows` and every row that descends from it, `root`
/// first. Each row appears once, whatever cycles the parent links hold.
pub(super) fn descendants(rows: &[ProcessRow], root: usize) -> Vec<usize> {
    let family = Family::of_all(rows);
    let mut found = Vec::new();
    let mut visited = Visited::new(rows.len());
    if root < rows.len() {
        family.descend(&[root], &mut visited, &mut |at, _| found.push(at));
    }
    found
}

/// The rows of `rows` that `pid`'s process descends from, nearest first, by
/// the same links the tree draws. Empty when `pid` is not listed.
pub(super) fn ancestors(rows: &[ProcessRow], pid: Pid) -> Vec<usize> {
    let family = Family::of_all(rows);
    let mut found = Vec::new();
    let mut visited = Visited::new(rows.len());
    let mut at = rows.iter().position(|row| row.sample().key.pid() == pid);
    while let Some(here) = at.filter(|&here| visited.insert(here)) {
        at = family.parent(here);
        found.extend(at.filter(|&up| !visited.contains(up)));
    }
    found
}

/// The positions of a list a walk has reached. A position past the end counts
/// as reached, so no walk ever steps outside the list.
struct Visited(Vec<bool>);

impl Visited {
    /// None of `len` positions reached yet.
    fn new(len: usize) -> Self {
        Self(vec![false; len])
    }

    fn contains(&self, at: usize) -> bool {
        self.0.get(at).copied().unwrap_or(true)
    }

    /// Marks `at` reached, and says whether it was not before.
    fn insert(&mut self, at: usize) -> bool {
        self.0
            .get_mut(at)
            .is_some_and(|reached| !std::mem::replace(reached, true))
    }
}

/// Parent and child links between listed rows, by position in the listing.
struct Family {
    parents: Vec<Option<usize>>,
    /// Each position's children, in listing order.
    children: Vec<Vec<usize>>,
}

impl Family {
    /// The links between every row of `rows`.
    fn of_all(rows: &[ProcessRow]) -> Self {
        let listed: Vec<&ProcessSample> = rows.iter().map(ProcessRow::sample).collect();
        Self::of(&listed)
    }

    /// The links between the processes `listed`, by position. A PID listed
    /// twice keeps its first position as the parent.
    fn of(listed: &[&ProcessSample]) -> Self {
        let mut by_pid = HashMap::with_capacity(listed.len());
        for (at, sample) in listed.iter().enumerate() {
            by_pid.entry(sample.key.pid()).or_insert(at);
        }
        let parents: Vec<Option<usize>> = listed
            .iter()
            .enumerate()
            .map(|(at, child)| {
                let up = *by_pid.get(&child.parent?)?;
                let parent = listed.get(up)?;
                let trusted = up != at && !started_after(parent.key.start(), child.key.start());
                trusted.then_some(up)
            })
            .collect();
        let mut children = vec![Vec::new(); listed.len()];
        for (at, parent) in parents.iter().enumerate() {
            if let Some(siblings) = parent.and_then(|up| children.get_mut(up)) {
                siblings.push(at);
            }
        }
        Self { parents, children }
    }

    /// The trusted parent of position `at`, if it has one.
    fn parent(&self, at: usize) -> Option<usize> {
        self.parents.get(at).copied().flatten()
    }

    /// The children of position `at`, in listing order.
    fn children(&self, at: usize) -> &[usize] {
        self.children.get(at).map(Vec::as_slice).unwrap_or_default()
    }

    /// Visits `roots` and everything below them depth first, in order,
    /// calling `visit` with each position and its tree line. Positions
    /// already visited are skipped, so a cycle ends the walk.
    fn descend(
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
    fn cycle_entry(&self, start: usize, visited: &Visited) -> usize {
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

/// Whether a claimed parent started after its child: then the parent's PID
/// was reused, and the link is stale. Unknown starts cannot tell.
fn started_after(parent: Start, child: Start) -> bool {
    matches!((parent, child), (Start::Known(parent), Start::Known(child)) if parent > child)
}
