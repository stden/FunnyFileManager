//! Purpose: unit tests for a renice step: from each target's own value, refused at
//! a bound or without a value to step from.

use super::*;

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
    assert_eq!(plan.changes().len(), 0);
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
