//! Purpose: Zero-clone open-addressed name index over `Panel::entries`.

use std::hash::{DefaultHasher, Hash, Hasher};

use super::super::collection::Revision;
use super::add_marked;
use crate::models::DirEntry;

/// `0` marks an unoccupied open-addressing slot; occupied slots store `bucket_index + 1`.
const EMPTY_SLOT: u32 = 0;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct MarkedCounts {
    pub(super) folders: usize,
    pub(super) files: usize,
    pub(super) bytes: u64,
}

impl MarkedCounts {
    pub(super) const fn into_tuple(self) -> (usize, usize, u64) {
        (self.folders, self.files, self.bytes)
    }
}

#[derive(Debug, Clone, Copy)]
struct NameBucket {
    first_entry_idx: usize,
    counts: MarkedCounts,
}

fn slot_to_bucket_index(slot: u32) -> usize {
    usize::try_from(slot - 1).unwrap_or(0)
}

/// Open-addressed hash table mapping distinct display names to aggregated [`MarkedCounts`].
#[derive(Debug, Clone)]
pub(super) struct NameIndex {
    pub(super) entries: Revision,
    slots: Vec<u32>,
    buckets: Vec<NameBucket>,
}

impl NameIndex {
    pub(super) fn build(entries_rev: Revision, entries: &[DirEntry]) -> Self {
        let cap = entries.len().saturating_mul(2).max(2).next_power_of_two();
        let mut index = Self {
            entries: entries_rev,
            slots: vec![EMPTY_SLOT; cap],
            buckets: Vec::with_capacity(entries.len()),
        };
        for (idx, entry) in entries.iter().enumerate() {
            index.insert_entry(entries, idx, entry);
        }
        index
    }

    fn insert_entry(&mut self, entries: &[DirEntry], idx: usize, entry: &DirEntry) {
        let mask = self.slots.len() - 1;
        let mut pos = hash_name(&entry.name) & mask;
        while !self.try_insert_at(entries, pos, idx, entry) {
            pos = (pos + 1) & mask;
        }
    }

    fn try_insert_at(
        &mut self,
        entries: &[DirEntry],
        pos: usize,
        idx: usize,
        entry: &DirEntry,
    ) -> bool {
        let slot = self.slots.get(pos).copied().unwrap_or(EMPTY_SLOT);
        if slot == EMPTY_SLOT {
            let next_slot = u32::try_from(self.buckets.len() + 1).unwrap_or(u32::MAX);
            if let Some(cell) = self.slots.get_mut(pos) {
                *cell = next_slot;
            }
            self.buckets.push(NameBucket {
                first_entry_idx: idx,
                counts: add_marked(MarkedCounts::default(), entry),
            });
            return true;
        }
        let matched = self
            .buckets
            .get_mut(slot_to_bucket_index(slot))
            .filter(|b| {
                entries
                    .get(b.first_entry_idx)
                    .is_some_and(|e| e.name == entry.name)
            });
        if let Some(bucket) = matched {
            bucket.counts = add_marked(bucket.counts, entry);
            return true;
        }
        false
    }

    pub(super) fn get(&self, entries: &[DirEntry], name: &str) -> Option<MarkedCounts> {
        let mask = self.slots.len() - 1;
        let mut pos = hash_name(name) & mask;
        loop {
            let slot = self.slots.get(pos).copied().unwrap_or(EMPTY_SLOT);
            if slot == EMPTY_SLOT {
                return None;
            }
            if let Some(counts) = self.counts_for_slot(entries, slot, name) {
                return Some(counts);
            }
            pos = (pos + 1) & mask;
        }
    }

    fn counts_for_slot(&self, entries: &[DirEntry], slot: u32, name: &str) -> Option<MarkedCounts> {
        let bucket = self.buckets.get(slot_to_bucket_index(slot))?;
        entries
            .get(bucket.first_entry_idx)
            .is_some_and(|e| e.name == name)
            .then_some(bucket.counts)
    }
}

fn hash_name(name: &str) -> usize {
    let mut hasher = DefaultHasher::new();
    name.hash(&mut hasher);
    usize::try_from(hasher.finish() & u64::from(u32::MAX)).unwrap_or(0)
}
