//! Purpose: a sample that keeps the same order does not rebuild the view, and
//! a sample that changes CPU order or a tree parent does.

use std::time::{Duration, Instant};

use super::tests_support::{child_of, counters, listing, process, sample, shown, system};
use super::view::VIEW_BUILDS;
use super::*;

fn apply(list: &mut TaskList, sample: Sample) {
    let now = Instant::now();
    list.take_request(now);
    list.apply_sample(Ok(sample), now);
}

fn builds() -> usize {
    VIEW_BUILDS.get()
}

#[test]
fn reuse_rejects_changed_options_without_changing_the_existing_view() {
    let list = listing(vec![process(1, "one"), process(2, "two")]);
    let mut view = list.view.clone();
    assert!(view::try_reuse(
        &mut view,
        list.table.rows(),
        &list.options,
        &list.view_stamp,
    ));
    let before = view.clone();
    let mut options = list.options.clone();
    options.arrangement = Arrangement::Tree;
    assert!(!view::try_reuse(
        &mut view,
        list.table.rows(),
        &options,
        &list.view_stamp,
    ));
    assert_eq!(view, before);
}

#[test]
fn an_unchanged_order_reuses_the_view_and_a_cpu_change_sorts_again() {
    let mut list = listing(vec![
        process(1, "one"),
        process(2, "two"),
        process(3, "three"),
    ]);
    let before = builds();
    apply(
        &mut list,
        sample(
            system(counters(1, 1, 1), &[counters(1, 1, 1)]),
            vec![process(1, "one"), process(2, "two"), process(3, "three")],
        ),
    );
    assert_eq!(shown(&list), vec![1, 2, 3]);
    assert_eq!(builds(), before, "the same order does not sort again");

    let mut busy = process(3, "three");
    busy.cpu_time = Duration::from_secs(1);
    apply(
        &mut list,
        sample(
            system(counters(3, 1, 1), &[counters(3, 1, 1)]),
            vec![process(1, "one"), process(2, "two"), busy],
        ),
    );
    assert_eq!(shown(&list).first().copied(), Some(3));
    assert!(builds() > before, "a new CPU order sorts again");
}

#[test]
fn a_tree_with_the_same_parents_is_kept_and_a_new_parent_is_rebuilt() {
    let mut list = listing(vec![process(1, "parent"), child_of(1, process(2, "child"))]);
    list.toggle_tree();
    let before = builds();
    let lines: Vec<_> = list.view().iter().map(|row| row.line()).collect();
    apply(
        &mut list,
        sample(
            system(counters(1, 1, 1), &[counters(1, 1, 1)]),
            vec![process(1, "parent"), child_of(1, process(2, "child"))],
        ),
    );
    let again: Vec<_> = list.view().iter().map(|row| row.line()).collect();
    assert_eq!(again, lines);
    assert_eq!(builds(), before);

    apply(
        &mut list,
        sample(
            system(counters(2, 1, 1), &[counters(2, 1, 1)]),
            vec![process(1, "parent"), process(2, "child")],
        ),
    );
    let parentless: Vec<_> = list.view().iter().map(|row| row.line()).collect();
    assert_ne!(parentless, lines);
    assert!(builds() > before);
}
