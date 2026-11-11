//! Purpose: Cached `O(G + M)` group-and-entry row layout for the unlocked vault tree view.

use std::cell::RefCell;
use std::collections::HashMap;

use super::Vault;
use crate::keepass::EntrySummary;

#[derive(Clone, Copy)]
pub enum VaultTreeRow {
    Group {
        group_idx: usize,
    },
    Entry {
        group_idx: usize,
        entry_idx: usize,
        match_pos: usize,
    },
}

pub struct VaultTreeLayout {
    pub rows: Vec<VaultTreeRow>,
    pub match_row: Vec<usize>,
}

pub(super) struct VaultTreeCache {
    entry_groups: Vec<Option<usize>>,
    layout: RefCell<Option<VaultTreeLayout>>,
}

impl VaultTreeCache {
    pub(super) fn new(entries: &[EntrySummary], groups: &[Vec<String>]) -> Self {
        Self {
            entry_groups: index_entry_groups(entries, groups),
            layout: RefCell::new(None),
        }
    }
}

fn index_entry_groups(entries: &[EntrySummary], groups: &[Vec<String>]) -> Vec<Option<usize>> {
    let mut by_path: HashMap<&[String], usize> = HashMap::with_capacity(groups.len());
    for (idx, group) in groups.iter().enumerate() {
        by_path.entry(group.as_slice()).or_insert(idx);
    }
    entries
        .iter()
        .map(|entry| by_path.get(entry.group_path.as_slice()).copied())
        .collect()
}

impl VaultTreeLayout {
    fn build(vault: &Vault) -> Self {
        let mut buckets: Vec<Vec<(usize, usize)>> = vec![Vec::new(); vault.groups.len()];
        for (match_pos, &entry_idx) in vault.matches.iter().enumerate() {
            if let Some(slot) = vault
                .tree_cache
                .entry_groups
                .get(entry_idx)
                .and_then(|&group_idx| group_idx)
                .and_then(|group_idx| buckets.get_mut(group_idx))
            {
                slot.push((match_pos, entry_idx));
            }
        }
        let mut rows = Vec::with_capacity(vault.groups.len() + vault.matches.len());
        let mut match_row = vec![0; vault.matches.len()];
        for (group_idx, bucket) in buckets.into_iter().enumerate() {
            rows.push(VaultTreeRow::Group { group_idx });
            append_bucket(&mut rows, &mut match_row, group_idx, bucket);
        }
        Self { rows, match_row }
    }
}

fn append_bucket(
    rows: &mut Vec<VaultTreeRow>,
    match_row: &mut [usize],
    group_idx: usize,
    bucket: Vec<(usize, usize)>,
) {
    for (match_pos, entry_idx) in bucket {
        if let Some(target) = match_row.get_mut(match_pos) {
            *target = rows.len();
        }
        rows.push(VaultTreeRow::Entry {
            group_idx,
            entry_idx,
            match_pos,
        });
    }
}

impl Vault {
    pub(crate) fn with_tree_layout<R>(&self, f: impl FnOnce(&VaultTreeLayout) -> R) -> R {
        let mut guard = self.tree_cache.layout.borrow_mut();
        let layout = guard.get_or_insert_with(|| VaultTreeLayout::build(self));
        f(layout)
    }

    pub(super) fn invalidate_tree_layout(&mut self) {
        self.tree_cache.layout = RefCell::new(None);
    }
}
