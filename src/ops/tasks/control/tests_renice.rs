//! Purpose: Tests for renicing through the task list, with a fixture host
//! that records instead of applying: `]` at once, `[` confirmed first unless
//! FFM is root, the bounds, the Priority form, and the next step starting
//! from the value the kernel accepted.

use crate::models::{
    ControlError, ControlRefusal, KernelPriority, Modal, NiceStep, Notice, Priority,
    PriorityChange, ProcessSample, ReniceButton, Schedule,
};
use crate::prompt::Privilege;
use crate::test_support::processes::Control;
use crate::test_support::task_list::{key_of, named, nice, TaskApp};

use super::{answer_renice, open_priority, set_nice, step_nice};

fn at_nice(raw: u32, name: &str, value: i32) -> ProcessSample {
    ProcessSample {
        priority: Priority::Unix {
            kernel: KernelPriority::new(20),
            nice: nice(value),
        },
        ..named(raw, name)
    }
}

#[test]
fn a_raise_renices_at_once_and_the_next_steps_from_the_accepted_value() {
    let mut tasks = TaskApp::open(vec![at_nice(4242, "firefox", 5)], Privilege::User);
    step_nice(&mut tasks.app, NiceStep::Raise);
    step_nice(&mut tasks.app, NiceStep::Raise);
    assert_eq!(
        tasks.host.controls(),
        [
            Control::Priority(key_of(4242), PriorityChange::Nice(nice(4))),
            Control::Priority(key_of(4242), PriorityChange::Nice(nice(3))),
        ]
    );
    assert_eq!(
        tasks.list().notice(),
        Some(Notice::Stepped {
            step: NiceStep::Raise,
            count: 1
        })
    );
    assert_eq!(tasks.list().refresh().schedule(), Schedule::Due);
}

#[test]
fn a_lower_asks_a_user_first_and_renices_only_when_confirmed() {
    let mut tasks = TaskApp::open(vec![at_nice(4242, "firefox", 5)], Privilege::User);
    step_nice(&mut tasks.app, NiceStep::Lower);
    assert!(matches!(tasks.list().modal(), Modal::Renice(_)));
    assert_eq!(tasks.host.controls().len(), 0, "nothing before the answer");
    answer_renice(&mut tasks.app, ReniceButton::Cancel);
    assert_eq!(tasks.list().modal(), &Modal::Browse);
    assert_eq!(tasks.host.controls().len(), 0);
    step_nice(&mut tasks.app, NiceStep::Lower);
    answer_renice(&mut tasks.app, ReniceButton::Renice);
    assert_eq!(
        tasks.host.controls(),
        [Control::Priority(
            key_of(4242),
            PriorityChange::Nice(nice(6))
        )]
    );
    answer_renice(&mut tasks.app, ReniceButton::Renice);
    assert_eq!(tasks.host.controls().len(), 1, "no second answer");
}

#[test]
fn a_lower_renices_root_at_once() {
    let mut tasks = TaskApp::open(vec![at_nice(4242, "firefox", 5)], Privilege::Root);
    step_nice(&mut tasks.app, NiceStep::Lower);
    assert_eq!(tasks.list().modal(), &Modal::Browse);
    assert_eq!(
        tasks.host.controls(),
        [Control::Priority(
            key_of(4242),
            PriorityChange::Nice(nice(6))
        )]
    );
}

#[test]
fn a_step_past_a_bound_is_a_notice_and_no_call_even_for_a_user() {
    let mut tasks = TaskApp::open(vec![at_nice(4242, "firefox", 19)], Privilege::User);
    step_nice(&mut tasks.app, NiceStep::Lower);
    assert_eq!(tasks.list().modal(), &Modal::Browse, "nothing to confirm");
    assert_eq!(
        tasks.list().notice(),
        Some(Notice::Refused(ControlRefusal::AtLimit(nice(19))))
    );
    assert_eq!(tasks.host.controls().len(), 0);
}

#[test]
fn a_refused_raise_is_worded_as_needing_root() {
    let mut tasks = TaskApp::open(vec![at_nice(4242, "firefox", 5)], Privilege::User);
    tasks
        .host
        .fail(key_of(4242), ControlError::PermissionDenied);
    step_nice(&mut tasks.app, NiceStep::Raise);
    let Modal::Error { failures, .. } = tasks.list().modal() else {
        panic!("the error dialog is open");
    };
    assert_eq!(
        failures[0].to_string(),
        "Only root can raise a process's priority (CAP_SYS_NICE)."
    );
}

/// Types `text` into the open Priority form's field.
fn type_nice(tasks: &mut TaskApp, text: &str) {
    let Modal::Priority(form) = tasks.list().modal_mut() else {
        panic!("the priority form is open");
    };
    form.value_mut().set_text(text);
}

#[test]
fn the_priority_form_offers_the_nice_value_and_sets_what_is_typed() {
    let mut tasks = TaskApp::open(vec![at_nice(4242, "firefox", 3)], Privilege::User);
    open_priority(tasks.list());
    let Modal::Priority(form) = tasks.list().modal() else {
        panic!("the priority form is open");
    };
    assert_eq!(form.value().text(), "3");
    type_nice(&mut tasks, "x");
    set_nice(&mut tasks.app);
    assert!(
        matches!(tasks.list().modal(), Modal::Priority(_)),
        "Set is disabled while the value is not a nice value"
    );
    type_nice(&mut tasks, "-2");
    set_nice(&mut tasks.app);
    assert_eq!(
        tasks.host.controls(),
        [Control::Priority(
            key_of(4242),
            PriorityChange::Nice(nice(-2))
        )]
    );
}

#[test]
fn a_set_nice_value_is_told_and_shown_until_the_next_sample() {
    let mut tasks = TaskApp::open(vec![at_nice(4242, "firefox", 3)], Privilege::User);
    open_priority(tasks.list());
    type_nice(&mut tasks, "-2");
    set_nice(&mut tasks.app);
    assert_eq!(
        tasks.list().notice(),
        Some(Notice::Reniced {
            count: 1,
            nice: nice(-2)
        })
    );
    let row = tasks
        .list()
        .row(key_of(4242))
        .map(|row| row.sample().priority);
    assert!(matches!(row, Some(Priority::Unix { nice: set, .. }) if set == nice(-2)));
}

#[test]
fn the_priority_form_for_nothing_is_a_notice() {
    let mut tasks = TaskApp::open(Vec::new(), Privilege::Root);
    open_priority(tasks.list());
    assert_eq!(tasks.list().modal(), &Modal::Browse);
    assert_eq!(
        tasks.list().notice(),
        Some(Notice::Refused(ControlRefusal::NoTarget))
    );
    set_nice(&mut tasks.app);
    answer_renice(&mut tasks.app, ReniceButton::Renice);
    assert_eq!(tasks.host.controls().len(), 0);
}
