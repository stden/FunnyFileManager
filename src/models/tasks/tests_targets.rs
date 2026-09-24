//! Purpose: Tests for freezing targets: the refusals before any system call,
//! FFM's ancestors, a cyclic parent chain, and targets that survive a
//! refresh that moves every row.

use super::targets::refusal;
use super::tests_support::{child_of, key, listing, pid, process, resample, OWN};
use super::{
    ControlRefusal, Cursor, Niceness, Operation, ProcessKey, ProcessKind, ProcessSample, Relation,
    SortKey, Start, Target, TaskList,
};

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

fn unknown_start(raw: u32) -> ProcessSample {
    ProcessSample {
        key: ProcessKey::new(pid(raw), Start::Unknown),
        ..process(raw, "hidden")
    }
}

fn kernel_thread(raw: u32) -> ProcessSample {
    ProcessSample {
        kind: ProcessKind::KernelThread,
        ..process(raw, "kworker")
    }
}

#[test]
fn signals_are_refused_to_ffm_init_kernel_threads_and_the_unverifiable() {
    let cases = [
        (process(OWN, "ffm"), Some(ControlRefusal::OwnProcess)),
        (process(1, "systemd"), Some(ControlRefusal::Init)),
        (process(2, "bash"), None),
        (kernel_thread(3), Some(ControlRefusal::KernelThread)),
        (unknown_start(4), Some(ControlRefusal::Unverifiable)),
    ];
    for (sample, expected) in cases {
        let raw = sample.key.pid();
        assert_eq!(
            refusal(&sample, Some(pid(OWN)), Operation::Signal),
            expected,
            "{raw}"
        );
    }
}

#[test]
fn a_renice_is_refused_only_when_the_process_cannot_be_confirmed() {
    for sample in [process(OWN, "ffm"), process(1, "systemd"), kernel_thread(3)] {
        assert_eq!(refusal(&sample, Some(pid(OWN)), Operation::Renice), None);
    }
    let unknown = unknown_start(4);
    assert_eq!(
        refusal(&unknown, Some(pid(OWN)), Operation::Renice),
        Some(ControlRefusal::Unverifiable)
    );
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

#[test]
fn nothing_left_to_do_is_explained_by_the_first_refusal() {
    let empty = listing(Vec::new()).freeze_targets(Operation::Signal);
    assert_eq!(empty.refusal(), Some(ControlRefusal::NoTarget));
    let init = listing(vec![process(1, "init")]).freeze_targets(Operation::Signal);
    assert_eq!(init.refusal(), Some(ControlRefusal::Init));
    let vim = listing(vec![process(2, "vim")]).freeze_targets(Operation::Signal);
    assert_eq!(vim.refusal(), None);
}

#[test]
fn every_refusal_reads_as_one_sentence() {
    let nineteen = Niceness::MAX;
    let sentences = [
        (
            ControlRefusal::OwnProcess,
            "This is FFM itself; quit with F10.",
        ),
        (
            ControlRefusal::Init,
            "PID 1 (init) is never signalled from here.",
        ),
        (
            ControlRefusal::KernelThread,
            "Kernel threads ignore signals.",
        ),
        (
            ControlRefusal::Unverifiable,
            "FFM cannot confirm which process this is; nothing was sent.",
        ),
        (ControlRefusal::AtLimit(nineteen), "Nice is already 19."),
        (
            ControlRefusal::RealtimeClass,
            "The Realtime class is not changed from here.",
        ),
        (ControlRefusal::NoTarget, "No process is selected."),
    ];
    for (refusal, sentence) in sentences {
        assert_eq!(refusal.to_string(), sentence);
    }
}

/// The platform asks the PID rules before its first system call, and the
/// kind rule once it has re-read the process; together they are the rule
/// the dialog freezes targets by.
#[test]
fn the_signal_rules_split_into_one_by_pid_and_one_by_kind() {
    let own = Some(pid(OWN));
    let by_pid = |raw| ControlRefusal::signal_to_pid(pid(raw), own);
    assert_eq!(by_pid(OWN), Some(ControlRefusal::OwnProcess));
    assert_eq!(by_pid(1), Some(ControlRefusal::Init));
    assert_eq!(by_pid(2), None);
    assert_eq!(ControlRefusal::signal_to_pid(pid(OWN), None), None);
    let by_kind = ControlRefusal::signal_to_kind;
    assert_eq!(
        by_kind(ProcessKind::KernelThread),
        Some(ControlRefusal::KernelThread)
    );
    assert_eq!(by_kind(ProcessKind::User), None);
}
