//! Purpose: Tests for the parent tree: guides, orphans, stale and cyclic
//! parent links, a chain too deep for recursion, and the descendant and
//! ancestor walks.

use super::tests_support::{child_of, key, pid, process, snapshot};
use super::tree::{ancestors, descendants};
use super::{
    build_view, Arrangement, ProcessSample, Sibling, Snapshot, Sort, SortKey, TreeLine, ViewOptions,
};
use crate::models::SortOrder;

fn tree_options() -> ViewOptions {
    ViewOptions {
        arrangement: Arrangement::Tree,
        sort: Sort::new(SortKey::Pid, SortOrder::Direct),
        ..ViewOptions::default()
    }
}

/// Each shown row as its PID and tree line, in view order.
fn tree(snapshot: &Snapshot) -> Vec<(u32, TreeLine)> {
    build_view(snapshot.rows(), &tree_options())
        .iter()
        .map(|shown| {
            let pid = snapshot.rows()[shown.row()].sample().key.pid().get();
            (pid, shown.line())
        })
        .collect()
}

const fn node(depth: u16, sibling: Sibling, open: u64) -> TreeLine {
    TreeLine::Node {
        depth,
        sibling,
        open,
    }
}

#[test]
fn children_sit_under_their_parent_with_guides() {
    // 1 ─┬─ 2 ── 4
    //    └─ 3
    let snapshot = snapshot(vec![
        process(1, "init"),
        child_of(1, process(2, "shell")),
        child_of(1, process(3, "cron")),
        child_of(2, process(4, "vim")),
    ]);
    assert_eq!(
        tree(&snapshot),
        [
            (1, node(0, Sibling::Last, 0)),
            (2, node(1, Sibling::Middle, 0)),
            (4, node(2, Sibling::Last, 0b1)),
            (3, node(1, Sibling::Last, 0)),
        ]
    );
}

#[test]
fn a_process_whose_parent_is_not_listed_is_a_root() {
    let snapshot = snapshot(vec![child_of(99, process(5, "orphan")), process(1, "init")]);
    assert_eq!(
        tree(&snapshot),
        [
            (1, node(0, Sibling::Middle, 0)),
            (5, node(0, Sibling::Last, 0))
        ]
    );
}

#[test]
fn a_parent_that_started_after_its_child_is_a_reused_pid() {
    let newcomer = ProcessSample {
        key: key(2, 500),
        ..process(2, "newcomer")
    };
    let child = ProcessSample {
        key: key(7, 100),
        ..child_of(2, process(7, "old-child"))
    };
    let snapshot = snapshot(vec![newcomer, child]);
    let depths: Vec<_> = tree(&snapshot)
        .into_iter()
        .map(|(pid, line)| (pid, matches!(line, TreeLine::Node { depth: 0, .. })))
        .collect();
    assert_eq!(depths, [(2, true), (7, true)]);
}

#[test]
fn a_self_parent_and_a_two_cycle_are_each_shown_once() {
    // Started in the same tick, so the start-time rule cannot break the cycle.
    let left = ProcessSample {
        key: key(4, 7),
        ..child_of(5, process(4, "left"))
    };
    let right = ProcessSample {
        key: key(5, 7),
        ..child_of(4, process(5, "right"))
    };
    let snapshot = snapshot(vec![child_of(3, process(3, "itself")), left, right]);
    assert_eq!(
        tree(&snapshot),
        [
            (3, node(0, Sibling::Last, 0)),
            (4, node(0, Sibling::Last, 0)),
            (5, node(1, Sibling::Last, 0)),
        ]
    );
}

#[test]
fn a_ten_thousand_deep_chain_arranges_without_recursion() {
    const DEPTH: u32 = 10_000;
    let chain: Vec<ProcessSample> = (1..=DEPTH)
        .map(|raw| match raw {
            1 => process(1, "root"),
            _ => child_of(raw - 1, process(raw, "link")),
        })
        .collect();
    let arranged = tree(&snapshot(chain));
    assert_eq!(arranged.len(), 10_000);
    let deepest = arranged.last().map(|(pid, line)| (*pid, *line));
    assert_eq!(deepest, Some((DEPTH, node(9_999, Sibling::Last, 0))));
}

/// A chain of 70 processes, each with a leaf sorted after the next link, so
/// every link below the root has a sibling still to come.
fn branching_chain() -> Snapshot {
    let mut processes = vec![process(1, "root")];
    for raw in 2..=70 {
        processes.push(child_of(raw - 1, process(raw, "link")));
        processes.push(child_of(raw - 1, process(1_000 + raw, "leaf")));
    }
    snapshot(processes)
}

#[test]
fn guides_stop_at_the_sixty_fourth_column() {
    let snapshot = branching_chain();
    let arranged = tree(&snapshot);
    let (_, deepest_link) = arranged
        .iter()
        .find(|(pid, _)| *pid == 70)
        .copied()
        .expect("the last link is shown");
    assert_eq!(deepest_link, node(69, Sibling::Middle, u64::MAX));
    assert!(deepest_link.continues(0));
    assert!(deepest_link.continues(63));
    assert!(!deepest_link.continues(64));
    assert!(!deepest_link.continues(u16::MAX));
    assert!(!TreeLine::Flat.continues(0));
}

#[test]
fn descendants_include_the_root_and_survive_a_cycle() {
    // One start tick for all three, so only the visited set ends the cycle.
    let same_tick = |raw, parent| ProcessSample {
        key: key(raw, 5),
        ..child_of(parent, process(raw, "looped"))
    };
    let snapshot = snapshot(vec![
        same_tick(1, 3),
        same_tick(2, 1),
        same_tick(3, 2),
        process(4, "stranger"),
    ]);
    let rows = snapshot.rows();
    let mut found: Vec<u32> = descendants(rows, 1)
        .into_iter()
        .map(|at| rows[at].sample().key.pid().get())
        .collect();
    assert_eq!(found.first(), Some(&2), "the root comes first");
    found.sort_unstable();
    assert_eq!(found, [1, 2, 3]);
    assert!(descendants(rows, 99).is_empty());
}

#[test]
fn ancestors_run_nearest_first_and_stop_at_a_cycle() {
    let same_tick = |raw, parent| ProcessSample {
        key: key(raw, 5),
        ..child_of(parent, process(raw, "looped"))
    };
    let snapshot = snapshot(vec![
        process(1, "init"),
        child_of(1, process(2, "sshd")),
        child_of(2, process(3, "bash")),
        same_tick(8, 9),
        same_tick(9, 8),
    ]);
    let rows = snapshot.rows();
    let pids_of = |found: Vec<usize>| -> Vec<u32> {
        found
            .into_iter()
            .map(|at| rows[at].sample().key.pid().get())
            .collect()
    };
    assert_eq!(pids_of(ancestors(rows, pid(3))), [2, 1]);
    assert_eq!(pids_of(ancestors(rows, pid(8))), [9]);
    assert!(ancestors(rows, pid(77)).is_empty());
}
