//! Purpose: Tests for planned renices: one step from each target's own value
//! and refused at a bound, one value for every target of the Priority form
//! with the direction each one moves, the value the form offers, the
//! confirmation `[` asks for, and `[`'s dependence on root.

use super::tests_support::{key, listing, process, OWN};
use super::{
    ControlRefusal, KernelPriority, NiceConfirm, NicePlan, NiceStep, Niceness, Operation, Priority,
    PriorityChange, PriorityClass, ProcessKey, ProcessSample, Renice, ReniceButton, SettableClass,
    Start, TaskList,
};
use crate::prompt::Privilege;

fn nice(value: i32) -> Niceness {
    Niceness::try_from(value).expect("a fixture nice value")
}

fn at_nice(raw: u32, name: &str, value: i32) -> ProcessSample {
    ProcessSample {
        priority: Priority::Unix {
            kernel: KernelPriority::new(20),
            nice: nice(value),
        },
        ..process(raw, name)
    }
}

fn with_priority(raw: u32, priority: Priority) -> ProcessSample {
    ProcessSample {
        priority,
        ..process(raw, "win")
    }
}

/// PIDs with the change each gets.
type Changes = Vec<(u32, PriorityChange)>;
/// PIDs with the reason each is left out.
type Refusals = Vec<(u32, ControlRefusal)>;

/// The PIDs a plan changes, with each change, and the PIDs it skips.
fn outline(plan: &NicePlan) -> (Changes, Refusals) {
    let changes = plan
        .changes()
        .iter()
        .map(|planned| (planned.target().key().pid().get(), planned.change()));
    let skipped = plan
        .skipped()
        .iter()
        .map(|skipped| (skipped.target().key().pid().get(), skipped.refusal()));
    (changes.collect(), skipped.collect())
}

fn marked_all(processes: Vec<ProcessSample>) -> TaskList {
    let mut list = listing(processes);
    list.invert_marks();
    list
}

#[test]
fn a_step_moves_each_target_from_its_own_value_and_stops_at_the_bounds() {
    let list = marked_all(vec![
        at_nice(2, "vim", 5),
        at_nice(3, "top", 19),
        at_nice(4, "cc", -20),
    ]);
    let lower = list.plan_step(NiceStep::Lower);
    assert_eq!(lower.renice(), Renice::Step(NiceStep::Lower));
    assert_eq!(
        outline(&lower),
        (
            vec![
                (2, PriorityChange::Nice(nice(6))),
                (4, PriorityChange::Nice(nice(-19)))
            ],
            vec![(3, ControlRefusal::AtLimit(Niceness::MAX))]
        )
    );
    let raise = list.plan_step(NiceStep::Raise);
    assert_eq!(
        outline(&raise).1,
        vec![(4, ControlRefusal::AtLimit(Niceness::MIN))]
    );
    assert!(raise
        .changes()
        .iter()
        .all(|planned| planned.direction() == NiceStep::Raise));
    assert_eq!(raise.refusal(), None);
}

#[test]
fn a_step_of_the_cursor_alone_at_its_bound_says_why_nothing_changes() {
    let plan = listing(vec![at_nice(3, "top", 19)]).plan_step(NiceStep::Lower);
    assert!(plan.changes().is_empty());
    assert_eq!(plan.refusal(), Some(ControlRefusal::AtLimit(Niceness::MAX)));
    let nothing = listing(Vec::new()).plan_step(NiceStep::Raise);
    assert_eq!(nothing.refusal(), Some(ControlRefusal::NoTarget));
}

#[test]
fn a_step_keeps_the_freezing_refusals_and_may_renice_ffm_itself() {
    let unknown = ProcessSample {
        key: ProcessKey::new(key(7, 7).pid(), Start::Unknown),
        ..process(7, "ghost")
    };
    let list = marked_all(vec![at_nice(OWN, "ffm", 0), unknown]);
    let plan = list.plan_step(NiceStep::Raise);
    assert_eq!(
        outline(&plan),
        (
            vec![(OWN, PriorityChange::Nice(nice(-1)))],
            vec![(7, ControlRefusal::Unverifiable)]
        )
    );
}

#[test]
fn a_step_refuses_what_it_cannot_step_from() {
    let cases = [
        (
            Priority::Windows {
                class: PriorityClass::Realtime,
            },
            Err(ControlRefusal::RealtimeClass),
        ),
        (
            Priority::Windows {
                class: PriorityClass::High,
            },
            Err(ControlRefusal::ClassAtLimit),
        ),
        (
            Priority::Windows {
                class: PriorityClass::Normal,
            },
            Ok(PriorityChange::Class(SettableClass::AboveNormal)),
        ),
        (Priority::Unknown, Err(ControlRefusal::UnknownPriority)),
    ];
    for (priority, expected) in cases {
        assert_eq!(priority.stepped(NiceStep::Raise), expected, "{priority:?}");
    }
    let list = listing(vec![with_priority(5, Priority::Unknown)]);
    assert_eq!(
        list.plan_step(NiceStep::Lower).refusal(),
        Some(ControlRefusal::UnknownPriority)
    );
}

#[test]
fn a_set_gives_every_target_one_value_and_knows_which_way_each_moves() {
    let list = marked_all(vec![at_nice(2, "vim", 5), at_nice(3, "top", -5)]);
    let targets = list.freeze_targets(Operation::Renice);
    let plan = list.plan_set(&targets, nice(0));
    assert_eq!(plan.renice(), Renice::Set(nice(0)));
    let moves: Vec<(u32, PriorityChange, NiceStep)> = plan
        .changes()
        .iter()
        .map(|planned| {
            let pid = planned.target().key().pid().get();
            (pid, planned.change(), planned.direction())
        })
        .collect();
    assert_eq!(
        moves,
        [
            (2, PriorityChange::Nice(nice(0)), NiceStep::Raise),
            (3, PriorityChange::Nice(nice(0)), NiceStep::Lower),
        ]
    );
    let same = list.plan_set(&targets, nice(5));
    assert_eq!(same.changes()[0].direction(), NiceStep::Lower, "no raise");
}

#[test]
fn a_set_for_a_process_gone_from_the_table_counts_as_lowering() {
    let list = listing(vec![at_nice(2, "vim", 5)]);
    let targets = list.freeze_targets(Operation::Renice);
    let later = listing(vec![at_nice(3, "top", 0)]);
    let plan = later.plan_set(&targets, nice(-3));
    assert_eq!(plan.changes()[0].direction(), NiceStep::Lower);
    assert_eq!(plan.changes()[0].target().key(), key(2, 2));
}

#[test]
fn a_set_carries_the_targets_the_form_left_out() {
    let list = listing(vec![ProcessSample {
        key: ProcessKey::new(key(7, 7).pid(), Start::Unknown),
        ..process(7, "ghost")
    }]);
    let targets = list.freeze_targets(Operation::Renice);
    let plan = list.plan_set(&targets, nice(1));
    assert_eq!(plan.refusal(), Some(ControlRefusal::Unverifiable));
}

#[test]
fn the_priority_form_offers_the_first_targets_nice_value() {
    let list = marked_all(vec![at_nice(2, "vim", 7), at_nice(3, "top", 1)]);
    let targets = list.freeze_targets(Operation::Renice);
    assert_eq!(list.offered_nice(&targets), nice(7));
    let windows = listing(vec![with_priority(
        4,
        Priority::Windows {
            class: PriorityClass::Idle,
        },
    )]);
    let targets = windows.freeze_targets(Operation::Renice);
    assert_eq!(windows.offered_nice(&targets), Niceness::NORMAL);
    let empty = listing(Vec::new());
    let none = empty.freeze_targets(Operation::Renice);
    assert_eq!(empty.offered_nice(&none), Niceness::NORMAL);
}

#[test]
fn the_renice_confirmation_focuses_renice_and_toggles_to_cancel() {
    let plan = listing(vec![at_nice(2, "vim", 0)]).plan_step(NiceStep::Lower);
    let mut confirm = NiceConfirm::new(plan.clone());
    assert_eq!(confirm.focused(), ReniceButton::Renice);
    assert_eq!(confirm.plan(), &plan);
    confirm.focus_other();
    assert_eq!(confirm.focused(), ReniceButton::Cancel);
    confirm.focus_other();
    assert_eq!(confirm.focused(), ReniceButton::Renice);
    assert_eq!(confirm.into_plan(), plan);
}

#[test]
fn a_list_knows_whether_ffm_runs_as_root_and_assumes_it_does_not() {
    assert_eq!(TaskList::default().privilege(), Privilege::User);
    assert_eq!(Privilege::default(), Privilege::User);
    let root = TaskList::new(
        super::TaskSession::new(1),
        key(OWN, 1).pid(),
        Privilege::Root,
    );
    assert_eq!(root.privilege(), Privilege::Root);
}
