//! Purpose: unit tests for a renice set: one value for every target of the Priority
//! form and the direction each one moves.

use super::*;

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
