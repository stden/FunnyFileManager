//! Purpose: orders a directory listing by the panel's sort mode, directories first.

use std::{cmp::Ordering, mem, path::Path};

use crate::models::{DirEntry, SortMode, SortOrder};

mod folded;
use folded::{cmp_folded, cmp_names_folded, fold, Folded};

/// Directories first, then the mode's own order within each group; `order`
/// turns each group around when reversed, but keeps the directories on top, as Far does.
pub(super) fn sort_entries(entries: &mut [DirEntry], sort_mode: SortMode, order: SortOrder) {
    match sort_mode {
        // A stable partition: the listing's own order survives within each group.
        SortMode::Unsorted => float_directories(entries),
        SortMode::Name => sort_by(entries, |_| ()),
        SortMode::Extension => sort_text(entries, |entry| extension_of(&entry.name)),
        SortMode::Size => sort_by_lazy(entries, |entry| std::cmp::Reverse(entry.size)),
        SortMode::ModificationTime => {
            sort_by_lazy(entries, |entry| std::cmp::Reverse(entry.modified));
        }
        SortMode::CreationTime => sort_by_lazy(entries, |entry| std::cmp::Reverse(entry.created)),
        SortMode::AccessTime => sort_by_lazy(entries, |entry| std::cmp::Reverse(entry.accessed)),
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

/// Directories first, then `key`, with precomputed case-folded names.
fn sort_by<K: Ord>(entries: &mut [DirEntry], key: impl Fn(&DirEntry) -> K) {
    let keys: Vec<K> = entries.iter().map(key).collect();
    let order = order_with(entries, &keys);
    place(entries, &order);
}

/// Directories first, then `key`, evaluating case-folded names lazily on ties.
fn sort_by_lazy<K: Ord>(entries: &mut [DirEntry], key: impl Fn(&DirEntry) -> K) {
    let keys: Vec<K> = entries.iter().map(key).collect();
    let mut order: Vec<usize> = (0..entries.len()).collect();
    order.sort_by(|&left, &right| cmp_at(entries, &keys, &[], left, right));
    place(entries, &order);
}

fn sort_text(entries: &mut [DirEntry], key: impl for<'a> Fn(&'a DirEntry) -> &'a str) {
    let order = {
        let keys: Vec<Folded<'_>> = entries.iter().map(|entry| fold(key(entry))).collect();
        order_with(entries, &keys)
    };
    place(entries, &order);
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
    dirs_first(a, b)
        .then_with(|| {
            keys.get(left)
                .zip(keys.get(right))
                .map_or(Ordering::Equal, |(lk, rk)| lk.cmp(rk))
        })
        .then_with(|| compare_tiebreak_names(folded, left, right, a, b))
}

fn compare_tiebreak_names(
    folded: &[Folded<'_>],
    left: usize,
    right: usize,
    a: &DirEntry,
    b: &DirEntry,
) -> Ordering {
    if let (Some(fa), Some(fb)) = (folded.get(left), folded.get(right)) {
        cmp_folded(fa, fb)
    } else {
        cmp_names_folded(&a.name, &b.name)
    }
}

fn dirs_first(left: &DirEntry, right: &DirEntry) -> Ordering {
    left.kind.is_dir().cmp(&right.kind.is_dir()).reverse()
}

fn place(entries: &mut [DirEntry], order: &[usize]) {
    let mut visited = vec![false; entries.len()];
    for start in 0..entries.len() {
        if visited.get(start).copied().unwrap_or(true) {
            continue;
        }
        if order.get(start).copied() == Some(start) {
            if let Some(flag) = visited.get_mut(start) {
                *flag = true;
            }
            continue;
        }
        place_cycle(entries, order, &mut visited, start);
    }
}

/// Permute one cycle in place by saving `entries[start]`, pulling each predecessor
/// `entries[order[cur]]` into `entries[cur]`, and closing the cycle with `saved`.
fn place_cycle(entries: &mut [DirEntry], order: &[usize], visited: &mut [bool], start: usize) {
    let saved = entries.get_mut(start).map(mem::take).unwrap_or_default();
    let mut cur = start;
    loop {
        if let Some(flag) = visited.get_mut(cur) {
            *flag = true;
        }
        let next = order.get(cur).copied();
        if next == Some(start) {
            if let Some(slot) = entries.get_mut(cur) {
                *slot = saved;
            }
            return;
        }
        let Some(source_idx) = next.filter(|&idx| visited.get(idx) == Some(&false)) else {
            return;
        };
        let moved = entries
            .get_mut(source_idx)
            .map(mem::take)
            .unwrap_or_default();
        if let Some(slot) = entries.get_mut(cur) {
            *slot = moved;
        }
        cur = source_idx;
    }
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

#[cfg(test)]
mod tests;
