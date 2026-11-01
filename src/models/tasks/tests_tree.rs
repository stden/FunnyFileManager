//! Purpose: Tests for the parent tree: guides, orphans, stale and cyclic
//! parent links, a chain too deep for recursion, and the descendant and
//! ancestor walks.

use super::tests_support::{child_of, key, pid, process, snapshot};
use super::tree::{ancestors, descendants};
use super::{
    build_view, Arrangement, ProcessSample, Sibling, Snapshot, Sort, SortKey, TreeLine, ViewOptions,
};
use crate::models::SortOrder;

mod tests_arrange;
mod tests_walks;
