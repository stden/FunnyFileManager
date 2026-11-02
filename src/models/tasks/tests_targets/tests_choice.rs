//! Purpose: unit tests for which processes are the targets: the cursor or the marks
//! in view order, FFM's ancestors, a cyclic parent chain, and targets that
//! survive a refresh that moves every row.

use super::*;

fn keys(list: &TaskList, operation: Operation) -> (Vec<u32>, Vec<(u32, ControlRefusal)>) {
    let frozen = list.freeze_targets(operation);
    let targets = frozen
        .targets()
        .iter()
        .map(|target| target.key().pid().get());
    let skipped = frozen
        .skipped()
        .iter()
        .map(|skipped| (skipped.target().key().pid().get(), skipped.refusal()));
    (targets.collect(), skipped.collect())
}

#[test]
fn the_cursor_process_is_the_target_when_nothing_is_marked() {
    let mut list = listing(vec![
        process(1, "init"),
        process(2, "vim"),
        process(OWN, "ffm"),
    ]);
    assert_eq!(
        keys(&list, Operation::Signal),
        (vec![], vec![(1, ControlRefusal::Init)])
    );
    list.select_index(1, 10);
    assert_eq!(keys(&list, Operation::Signal), (vec![2], vec![]));
}

#[test]
fn marked_processes_are_the_targets_in_view_order_with_refusals_apart() {
    let mut list = listing(vec![
        process(3, "vim"),
        process(OWN, "ffm"),
        process(2, "less"),
    ]);
    list.choose_sort(SortKey::Pid);
    list.choose_sort(SortKey::Pid);
    list.select_index(0, 10);
    for _ in 0..3 {
        list.toggle_mark(10);
    }
    let (targets, skipped) = keys(&list, Operation::Signal);
    assert_eq!(targets, [3, 2]);
    assert_eq!(skipped, [(OWN, ControlRefusal::OwnProcess)]);
    assert_eq!(keys(&list, Operation::Renice).0, [OWN, 3, 2]);
}

#[test]
fn ffm_and_the_processes_it_runs_inside_are_named_as_such() {
    let mut list = listing(vec![
        process(1, "init"),
        child_of(1, process(10, "sshd")),
        child_of(10, process(20, "bash")),
        child_of(20, process(OWN, "ffm")),
        child_of(20, process(30, "vim")),
    ]);
    for _ in 0..5 {
        list.toggle_mark(10);
    }
    let frozen = list.freeze_targets(Operation::Renice);
    let relations: Vec<(&str, Relation)> = frozen
        .targets()
        .iter()
        .map(|target| (target.name(), target.relation()))
        .collect();
    let (above, other) = (Relation::Ancestor, Relation::Other);
    let expected = [
        ("init", above),
        ("sshd", above),
        ("bash", above),
        ("vim", other),
    ];
    assert_eq!(relations[..4], expected);
    assert_eq!(relations[4], ("ffm", Relation::Itself));
}

#[test]
fn a_cyclic_parent_chain_above_ffm_ends() {
    let same_tick = |raw, parent| ProcessSample {
        key: key(raw, 5),
        ..child_of(parent, process(raw, "looped"))
    };
    let mut list = listing(vec![same_tick(7, 8), same_tick(8, 7), same_tick(OWN, 7)]);
    list.toggle_mark(10);
    let frozen = list.freeze_targets(Operation::Signal);
    let relations: Vec<Relation> = frozen.targets().iter().map(Target::relation).collect();
    assert_eq!(relations, [Relation::Ancestor]);
}

#[test]
fn frozen_targets_survive_a_refresh_that_moves_every_row() {
    let mut list = listing(vec![process(5, "vim"), process(6, "less")]);
    list.select_index(1, 10);
    let frozen = list.freeze_targets(Operation::Signal);
    resample(
        &mut list,
        vec![process(4, "new"), process(6, "less"), process(5, "vim")],
    );
    assert_eq!(list.cursor().map(Cursor::index), Some(2));
    let targets: Vec<ProcessKey> = frozen.targets().iter().map(Target::key).collect();
    assert_eq!(targets, [key(6, 6)]);
}
