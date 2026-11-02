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

mod family;

use super::view::ViewRow;
use super::{Pid, ProcessRow, ProcessSample};

use family::Family;

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
    let family = Family::parents_of_all(rows);
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
