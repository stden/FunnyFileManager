//! Purpose: quick search by a sorted prefix index, still walking listing order.
//!
//! The index answers "which rows start with this prefix?" in O(log n). The
//! match itself is the earliest listing index in that range, then the next
//! one after the cursor, wrapping to the earliest. Alphabetical order of the
//! index is never the order the cursor moves in.

use super::Panel;
use crate::casefold::folded_name;
use crate::models::DirEntry;
use std::sync::Arc;

#[cfg(test)]
thread_local! {
    pub static INDEX_BUILDS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[derive(Debug, Clone)]
pub(super) struct NameIndex {
    key: Arc<()>,
    rows: Vec<(String, usize)>,
}

pub(super) fn select_prefixed(panel: &mut Panel, prefix: &str, next: bool) -> bool {
    if panel.entries.is_empty() || prefix.is_empty() {
        return false;
    }
    ensure(panel);
    let found = panel
        .name_index
        .borrow()
        .as_ref()
        .and_then(|index| choose(index, prefix, panel.selected().unwrap_or(0), next));
    if let Some(index) = found {
        panel.select(Some(index));
        return true;
    }
    false
}

/// Exact directory membership shares quick search's index across Git replies.
pub(super) fn contains_directory(panel: &Panel, name: &str) -> bool {
    ensure(panel);
    let index = panel.name_index.borrow();
    let Some(index) = index.as_ref() else {
        return false;
    };
    let folded = folded_name(name);
    let start = index.rows.partition_point(|(key, _)| key < &folded);
    index
        .rows
        .get(start..)
        .unwrap_or(&[])
        .iter()
        .take_while(|(key, _)| key == &folded)
        .any(|(_, row)| {
            panel
                .entries
                .get(*row)
                .is_some_and(|entry| entry.kind.is_dir() && entry.name == name)
        })
}

fn ensure(panel: &Panel) {
    let key = panel.entries.revision();
    let fresh = panel
        .name_index
        .borrow()
        .as_ref()
        .is_some_and(|index| Arc::ptr_eq(&index.key, key));
    if fresh {
        return;
    }
    *panel.name_index.borrow_mut() = Some(NameIndex::build(&panel.entries, Arc::clone(key)));
}

impl NameIndex {
    fn build(entries: &[DirEntry], key: Arc<()>) -> Self {
        #[cfg(test)]
        INDEX_BUILDS.set(INDEX_BUILDS.get() + 1);
        let mut rows = Vec::new();
        for (index, entry) in entries.iter().enumerate() {
            if entry.name != ".." {
                rows.push((folded_name(&entry.name), index));
            }
        }
        rows.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
        Self { key, rows }
    }
}

fn choose(index: &NameIndex, prefix: &str, current: usize, next: bool) -> Option<usize> {
    let rows = prefix_rows(&index.rows, prefix);
    if next {
        earliest(rows, Some(current)).or_else(|| earliest(rows, None))
    } else {
        earliest(rows, None)
    }
}

fn prefix_rows<'a>(rows: &'a [(String, usize)], prefix: &str) -> &'a [(String, usize)] {
    let folded = folded_name(prefix);
    let start = rows.partition_point(|(name, _)| name.as_str() < folded.as_str());
    let rest = rows.get(start..).unwrap_or(&[]);
    let count = rest.partition_point(|(name, _)| name.starts_with(&folded));
    rest.get(..count).unwrap_or(rest)
}

fn earliest(rows: &[(String, usize)], after: Option<usize>) -> Option<usize> {
    rows.iter()
        .filter(|(_, index)| after.is_none_or(|current| *index > current))
        .map(|(_, index)| *index)
        .min()
}
