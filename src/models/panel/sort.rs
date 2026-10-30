//! Purpose: orders a directory listing by the panel's sort mode, directories first.

use std::{cmp::Reverse, path::Path};

use crate::models::{DirEntry, SortMode, SortOrder};

/// Directories first, then the mode's own order within each group; `order`
/// turns each group around when reversed, but keeps the directories on top, as Far does.
pub(super) fn sort_entries(entries: &mut [DirEntry], sort_mode: SortMode, order: SortOrder) {
    match sort_mode {
        // Stable, so the listing's own order survives within each group.
        SortMode::Unsorted => entries.sort_by_key(|e| !e.kind.is_dir()),
        SortMode::Name => sort_by(entries, |_| ()),
        SortMode::Extension => sort_by(entries, |e| extension(&e.name)),
        SortMode::Size => sort_by(entries, |e| Reverse(e.size)),
        SortMode::ModificationTime => sort_by(entries, |e| Reverse(e.modified)),
        SortMode::CreationTime => sort_by(entries, |e| Reverse(e.created)),
        SortMode::AccessTime => sort_by(entries, |e| Reverse(e.accessed)),
        SortMode::Descriptions => sort_by(entries, |e| lowercase(e.description.as_deref())),
        SortMode::Owner => sort_by(entries, |e| lowercase(e.owner.as_deref())),
    }
    if order.is_reversed() {
        let directories = entries.iter().take_while(|e| e.kind.is_dir()).count();
        let (top, rest) = entries.split_at_mut(directories);
        top.reverse();
        rest.reverse();
    }
}

/// Directories first, then `key`, then the lowercased name as the tie-break.
///
/// `sort_by_cached_key` rather than a comparator, and for one reason: the
/// tie-break lowercases the name, and doing that inside a comparator
/// allocates two `String`s per comparison — about a quarter of a million of
/// them for a directory of ten thousand files. Cached, each name is
/// lowercased once.
fn sort_by<K: Ord>(entries: &mut [DirEntry], key: impl Fn(&DirEntry) -> K) {
    entries.sort_by_cached_key(|e| (!e.kind.is_dir(), key(e), e.name.to_lowercase()));
}

fn extension(name: &str) -> String {
    Path::new(name)
        .extension()
        .and_then(|x| x.to_str())
        .unwrap_or("")
        .to_lowercase()
}

/// Rows without the fact sort together, before those that have it.
fn lowercase(text: Option<&str>) -> String {
    text.unwrap_or_default().to_lowercase()
}
