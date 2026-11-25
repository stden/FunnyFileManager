//! Purpose: quick search by a sorted prefix index, still walking listing order.
//!
//! The index answers "which rows start with this prefix?" in O(log n). The
//! match itself is the earliest listing index in that range, then the next
//! one after the cursor, wrapping to the earliest. Alphabetical order of the
//! index is never the order the cursor moves in.

use super::Panel;
use crate::casefold::{folded_name, PrefixIndex};
use crate::models::DirEntry;
use std::sync::Arc;

#[cfg(test)]
thread_local! {
    pub static INDEX_BUILDS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[derive(Debug, Clone)]
pub(super) struct NameIndex {
    key: Arc<()>,
    tree: PrefixIndex,
}

pub(super) fn select_prefixed(panel: &mut Panel, prefix: &str, next: bool) -> bool {
    if panel.entries.is_empty() || prefix.is_empty() {
        return false;
    }
    let current = panel.selected().unwrap_or(0);
    let found = with_index(panel, |index| index.tree.choose(prefix, current, next));
    if let Some(index) = found {
        panel.select(Some(index));
        return true;
    }
    false
}

/// Exact directory membership shares quick search's index across Git replies.
pub(super) fn contains_directory(panel: &Panel, name: &str) -> bool {
    let folded = folded_name(name);
    with_index(panel, |index| {
        index.tree.exact_indices(&folded).any(|row| {
            panel
                .entries
                .get(row)
                .is_some_and(|entry| entry.kind.is_dir() && entry.name == name)
        })
    })
}

fn with_index<R>(panel: &Panel, f: impl FnOnce(&NameIndex) -> R) -> R {
    let key = panel.entries.revision();
    let mut guard = panel.name_index.borrow_mut();
    if guard
        .as_ref()
        .is_none_or(|index| !Arc::ptr_eq(&index.key, key))
    {
        *guard = None;
    }
    let index = guard.get_or_insert_with(|| NameIndex::build(&panel.entries, Arc::clone(key)));
    f(index)
}

impl NameIndex {
    fn build(entries: &[DirEntry], key: Arc<()>) -> Self {
        #[cfg(test)]
        INDEX_BUILDS.set(INDEX_BUILDS.get() + 1);
        let mut rows = Vec::with_capacity(entries.len());
        for (index, entry) in entries.iter().enumerate() {
            if entry.name != ".." {
                rows.push((folded_name(&entry.name), index));
            }
        }
        Self {
            key,
            tree: PrefixIndex::new(rows),
        }
    }
}
