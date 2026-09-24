//! Purpose: unit tests for the value the Priority form offers, the confirmation `[`
//! asks for, and `[`'s dependence on root.

use super::*;

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
    let root = TaskList::new(TaskSession::new(1), key(OWN, 1).pid(), Privilege::Root);
    assert_eq!(root.privilege(), Privilege::Root);
}
