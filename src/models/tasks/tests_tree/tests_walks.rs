//! Purpose: unit tests for the descendant and ancestor walks.

use super::*;

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
    assert_eq!(descendants(rows, 99).len(), 0);
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
    assert_eq!(ancestors(rows, pid(77)).len(), 0);
}
