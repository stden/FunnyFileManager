//! Purpose: the faster name comparison and the unsorted partition keep Far's
//! order — directories first, stable ties, reversal inside each group.

use std::cmp::Reverse;

use super::sort::sort_entries;
use super::*;
use crate::models::EntryKind;

fn entry(name: &str, directory: bool) -> DirEntry {
    DirEntry {
        name: name.to_owned(),
        kind: if directory {
            EntryKind::Directory
        } else {
            EntryKind::File
        },
        size: FileSize::from_bytes(u64::try_from(name.len()).unwrap_or(0)),
        description: (!name.is_empty()).then(|| format!("Note {name}")),
        owner: Some(format!("Owner {name}")),
        ..DirEntry::default()
    }
}

fn sample() -> Vec<DirEntry> {
    vec![
        entry("b", false),
        entry("Dir", true),
        entry("Copy.TXT", false),
        entry("copy.txt", false),
        entry("İstanbul", false),
        entry("istanbul", false),
        entry("dir", true),
        entry("B", false),
        entry("ıota", false),
        entry("ALPHA", false),
    ]
}

fn names(entries: &[DirEntry]) -> Vec<&str> {
    entries.iter().map(|entry| entry.name.as_str()).collect()
}

/// The previous comparator: lowercase every name inside the cached key.
fn reference(entries: &mut [DirEntry], mode: SortMode, order: SortOrder) {
    match mode {
        SortMode::Unsorted => entries.sort_by_key(|entry| !entry.kind.is_dir()),
        SortMode::Name => cache(entries, |_| ()),
        SortMode::Extension => cache(entries, |entry| extension_of(&entry.name)),
        SortMode::Size => cache(entries, |entry| Reverse(entry.size)),
        SortMode::ModificationTime => cache(entries, |entry| Reverse(entry.modified)),
        SortMode::CreationTime => cache(entries, |entry| Reverse(entry.created)),
        SortMode::AccessTime => cache(entries, |entry| Reverse(entry.accessed)),
        SortMode::Descriptions => cache(entries, |entry| folded(entry.description.as_deref())),
        SortMode::Owner => cache(entries, |entry| folded(entry.owner.as_deref())),
    }
    if order.is_reversed() {
        let directories = entries
            .iter()
            .take_while(|entry| entry.kind.is_dir())
            .count();
        let (top, rest) = entries.split_at_mut(directories);
        top.reverse();
        rest.reverse();
    }
}

fn cache<K: Ord>(entries: &mut [DirEntry], key: impl Fn(&DirEntry) -> K) {
    entries
        .sort_by_cached_key(|entry| (!entry.kind.is_dir(), key(entry), entry.name.to_lowercase()));
}

fn extension_of(name: &str) -> String {
    std::path::Path::new(name)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase()
}

fn folded(text: Option<&str>) -> String {
    text.unwrap_or_default().to_lowercase()
}

#[test]
fn every_mode_matches_the_lowercase_comparator_including_ties_and_reversal() {
    for mode in SortMode::ALL {
        for reversed in [false, true] {
            let order = SortOrder::from_reversed(reversed);
            let mut fast = sample();
            let mut slow = sample();
            sort_entries(&mut fast, mode, order);
            reference(&mut slow, mode, order);
            assert_eq!(names(&fast), names(&slow), "{mode:?} reversed={reversed}");
        }
    }
}
