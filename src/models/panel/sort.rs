//! Purpose: orders a directory listing by the panel's sort mode, directories first.

use std::{cmp::Ordering, mem, path::Path};

use crate::models::{DirEntry, SortMode, SortOrder};

/// Directories first, then the mode's own order within each group; `order`
/// turns each group around when reversed, but keeps the directories on top, as Far does.
pub(super) fn sort_entries(entries: &mut [DirEntry], sort_mode: SortMode, order: SortOrder) {
    match sort_mode {
        // A stable partition: the listing's own order survives within each group.
        SortMode::Unsorted => float_directories(entries),
        SortMode::Name => sort_by(entries, |_| ()),
        SortMode::Extension => sort_text(entries, |entry| extension_of(&entry.name)),
        SortMode::Size => sort_by(entries, |entry| std::cmp::Reverse(entry.size)),
        SortMode::ModificationTime => sort_by(entries, |entry| std::cmp::Reverse(entry.modified)),
        SortMode::CreationTime => sort_by(entries, |entry| std::cmp::Reverse(entry.created)),
        SortMode::AccessTime => sort_by(entries, |entry| std::cmp::Reverse(entry.accessed)),
        SortMode::Descriptions => sort_text(entries, |entry| text_of(entry.description.as_deref())),
        SortMode::Owner => sort_text(entries, |entry| text_of(entry.owner.as_deref())),
    }
    if order.is_reversed() {
        reverse_groups(entries);
    }
}

/// Directories first, preserving the relative order inside each group.
fn float_directories(entries: &mut [DirEntry]) {
    let mut directories = Vec::new();
    let mut files = Vec::new();
    for entry in entries.iter_mut() {
        let item = mem::take(entry);
        if item.kind.is_dir() {
            directories.push(item);
        } else {
            files.push(item);
        }
    }
    let mut restored = directories.into_iter().chain(files);
    for slot in entries.iter_mut() {
        if let Some(item) = restored.next() {
            *slot = item;
        }
    }
}

fn reverse_groups(entries: &mut [DirEntry]) {
    let directories = entries
        .iter()
        .take_while(|entry| entry.kind.is_dir())
        .count();
    let (top, rest) = entries.split_at_mut(directories);
    top.reverse();
    rest.reverse();
}

/// Directories first, then `key`, then the case-folded name.
///
/// The mode key is computed once per entry. ASCII names compare without
/// allocating; a name that is not ASCII is lowercased once, not once per
/// comparison. The sort is stable, so equal folded names keep listing order.
fn sort_by<K: Ord>(entries: &mut [DirEntry], key: impl Fn(&DirEntry) -> K) {
    let order = order_of(entries, key);
    place(entries, &order);
}

fn order_of<K: Ord>(entries: &[DirEntry], key: impl Fn(&DirEntry) -> K) -> Vec<usize> {
    let keys: Vec<K> = entries.iter().map(key).collect();
    order_with(entries, &keys)
}

fn sort_text(entries: &mut [DirEntry], key: impl for<'a> Fn(&'a DirEntry) -> &'a str) {
    let order = text_order(entries, &key);
    place(entries, &order);
}

fn text_order<'a>(entries: &'a [DirEntry], key: &impl Fn(&'a DirEntry) -> &'a str) -> Vec<usize> {
    let keys: Vec<Folded<'a>> = entries.iter().map(|entry| fold(key(entry))).collect();
    order_with(entries, &keys)
}

fn order_with<K: Ord>(entries: &[DirEntry], keys: &[K]) -> Vec<usize> {
    let folded: Vec<Folded<'_>> = entries.iter().map(|entry| fold(&entry.name)).collect();
    let mut order: Vec<usize> = (0..entries.len()).collect();
    order.sort_by(|&left, &right| cmp_at(entries, keys, &folded, left, right));
    order
}

fn cmp_at<K: Ord>(
    entries: &[DirEntry],
    keys: &[K],
    folded: &[Folded<'_>],
    left: usize,
    right: usize,
) -> Ordering {
    let (Some(a), Some(b)) = (entries.get(left), entries.get(right)) else {
        return left.cmp(&right);
    };
    let by_key = keys
        .get(left)
        .zip(keys.get(right))
        .map_or(Ordering::Equal, |(a, b)| a.cmp(b));
    let by_name = folded
        .get(left)
        .zip(folded.get(right))
        .map_or(Ordering::Equal, |(a, b)| cmp_folded(a, b));
    dirs_first(a, b).then(by_key).then(by_name)
}

fn dirs_first(left: &DirEntry, right: &DirEntry) -> Ordering {
    left.kind.is_dir().cmp(&right.kind.is_dir()).reverse()
}

fn place(entries: &mut [DirEntry], order: &[usize]) {
    let mut taken: Vec<DirEntry> = entries.iter_mut().map(mem::take).collect();
    for (dest, src) in order.iter().enumerate() {
        let (Some(slot), Some(item)) = (entries.get_mut(dest), taken.get_mut(*src)) else {
            continue;
        };
        *slot = mem::take(item);
    }
}

/// A name prepared for one sort. ASCII keeps the original text and is folded
/// while comparing; other names are lowercased once.
enum Folded<'a> {
    Ascii(&'a str),
    Unicode(String),
}

impl PartialEq for Folded<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Folded<'_> {}

impl PartialOrd for Folded<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Folded<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        cmp_folded(self, other)
    }
}

fn fold(name: &str) -> Folded<'_> {
    if name.is_ascii() {
        Folded::Ascii(name)
    } else {
        Folded::Unicode(name.to_lowercase())
    }
}

fn cmp_folded(left: &Folded<'_>, right: &Folded<'_>) -> Ordering {
    ascii_cmp(stored_bytes(left), stored_bytes(right))
}

fn stored_bytes<'a>(folded: &'a Folded<'_>) -> &'a [u8] {
    match folded {
        Folded::Ascii(text) => *text,
        Folded::Unicode(text) => text,
    }
    .as_bytes()
}

/// Both sides are folded here. An ASCII name is still the original text, so
/// its letters are lowercased during the compare. A Unicode name was already
/// passed through `to_lowercase`, and `to_ascii_lowercase` leaves those bytes.
fn ascii_cmp(left: &[u8], right: &[u8]) -> Ordering {
    left.iter()
        .copied()
        .map(|byte| byte.to_ascii_lowercase())
        .cmp(right.iter().copied().map(|byte| byte.to_ascii_lowercase()))
}

fn extension_of(name: &str) -> &str {
    Path::new(name)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
}

fn text_of(text: Option<&str>) -> &str {
    text.unwrap_or_default()
}
