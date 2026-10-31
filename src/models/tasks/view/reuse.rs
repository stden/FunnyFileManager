//! Purpose: keep the task list's row order when a new sample does not change
//! which processes are shown or how they sort. Figures still come from the
//! new snapshot, through updated row indexes.

use std::cmp::Ordering;
use std::collections::HashMap;

use super::super::{Pid, ProcessKey, ProcessRow};
use super::{Arrangement, ViewOptions, ViewRow};

/// What the current view was built from. A later sample may reuse the view
/// when these still describe it.
#[derive(Clone, Debug, Default)]
pub struct ViewStamp {
    options: ViewOptions,
    parents: Vec<Option<Pid>>,
}

pub fn try_reuse(
    view: &mut [ViewRow],
    rows: &[ProcessRow],
    options: &ViewOptions,
    stamp: &ViewStamp,
) -> bool {
    if stamp.options != *options {
        return false;
    }
    let indexes = index_of(rows, options);
    if indexes.len() != view.len() || !still_sorted(view, rows, options, &indexes) {
        return false;
    }
    if !parents_hold(view, rows, options, stamp, &indexes) {
        return false;
    }
    retarget(view, &indexes);
    true
}

pub fn stamp(view: &[ViewRow], rows: &[ProcessRow], options: &ViewOptions) -> ViewStamp {
    let parents = if options.arrangement == Arrangement::Tree {
        view.iter()
            .map(|slot| rows.get(slot.row()).and_then(|row| row.sample().parent))
            .collect()
    } else {
        Vec::new()
    };
    ViewStamp {
        options: options.clone(),
        parents,
    }
}

fn index_of(rows: &[ProcessRow], options: &ViewOptions) -> HashMap<ProcessKey, usize> {
    let mut indexes = HashMap::new();
    for (index, row) in rows.iter().enumerate() {
        if options.admits(row.sample()) {
            indexes.insert(row.sample().key, index);
        }
    }
    indexes
}

fn still_sorted(
    view: &[ViewRow],
    rows: &[ProcessRow],
    options: &ViewOptions,
    indexes: &HashMap<ProcessKey, usize>,
) -> bool {
    let mut previous: Option<usize> = None;
    for slot in view {
        let Some(&index) = indexes.get(&slot.key()) else {
            return false;
        };
        if let Some(before) = previous {
            let ordered = rows.get(before).zip(rows.get(index));
            let Some((left, right)) = ordered else {
                return false;
            };
            if options.sort.compare(left, right) == Ordering::Greater {
                return false;
            }
        }
        previous = Some(index);
    }
    true
}

fn parents_hold(
    view: &[ViewRow],
    rows: &[ProcessRow],
    options: &ViewOptions,
    stamp: &ViewStamp,
    indexes: &HashMap<ProcessKey, usize>,
) -> bool {
    if options.arrangement != Arrangement::Tree {
        return true;
    }
    if stamp.parents.len() != view.len() {
        return false;
    }
    view.iter().zip(&stamp.parents).all(|(slot, expected)| {
        let parent = indexes
            .get(&slot.key())
            .and_then(|index| rows.get(*index))
            .and_then(|row| row.sample().parent);
        parent == *expected
    })
}

fn retarget(view: &mut [ViewRow], indexes: &HashMap<ProcessKey, usize>) {
    for slot in view.iter_mut() {
        if let Some(&index) = indexes.get(&slot.key()) {
            *slot = slot.retarget(index);
        }
    }
}
