//! Purpose: Tests for signalling through the task list, with a fixture
//! host that records instead of sending: confirmations that freeze their
//! targets, a signal only after a button, the refusals, and every outcome
//! told as a notice or the Error dialog.

use crate::models::{
    ConfirmButton, ControlError, ControlRefusal, KillOffer, Modal, NiceStep, Notice, Schedule,
    Signal,
};
use crate::prompt::Privilege;
use crate::test_support::processes::Control;
use crate::test_support::task_list::{key_of, named, TaskApp, OWN};

use super::{confirm_terminate, open_signal_menu, press, step_nice};

fn firefox() -> TaskApp {
    TaskApp::open(vec![named(4242, "firefox")], Privilege::User)
}

#[test]
fn f8_then_terminate_signals_the_cursor_process_once() {
    let mut tasks = firefox();
    confirm_terminate(tasks.list());
    let Modal::Confirm(confirm) = tasks.list().modal() else {
        panic!("the confirmation is open");
    };
    assert_eq!(confirm.offer(), KillOffer::TerminateOrKill);
    assert!(tasks.host.controls().is_empty(), "opening sends nothing");
    press(&mut tasks.app, ConfirmButton::Terminate);
    assert_eq!(
        tasks.host.controls(),
        [Control::Signal(key_of(4242), Signal::Terminate)]
    );
    assert_eq!(tasks.list().modal(), &Modal::Browse);
    assert_eq!(
        tasks.list().notice(),
        Some(Notice::Sent {
            signal: Signal::Terminate,
            count: 1
        })
    );
    assert_eq!(tasks.list().refresh().schedule(), Schedule::Due);
}

#[cfg(unix)]
#[test]
fn kill_sends_sigkill_and_cancel_sends_nothing() {
    let mut tasks = firefox();
    confirm_terminate(tasks.list());
    press(&mut tasks.app, ConfirmButton::Cancel);
    assert!(tasks.host.controls().is_empty());
    assert_eq!(tasks.list().modal(), &Modal::Browse);
    assert_eq!(tasks.list().notice(), None);
    confirm_terminate(tasks.list());
    press(&mut tasks.app, ConfirmButton::Kill);
    assert_eq!(
        tasks.host.controls(),
        [Control::Signal(key_of(4242), Signal::Kill)]
    );
}

#[cfg(unix)]
#[test]
fn a_button_the_confirmation_lacks_does_nothing() {
    let mut tasks = firefox();
    open_signal_menu(tasks.list());
    let Modal::Menu(crate::models::TaskMenu::Signal { targets, .. }) = tasks.list().modal().clone()
    else {
        panic!("the signal menu is open");
    };
    let confirm = crate::models::KillConfirm::new(targets, KillOffer::Send(Signal::Hangup));
    tasks.list().set_modal(Modal::Confirm(confirm));
    press(&mut tasks.app, ConfirmButton::Kill);
    assert!(tasks.host.controls().is_empty());
    assert!(
        matches!(tasks.list().modal(), Modal::Confirm(_)),
        "still open"
    );
    press(&mut tasks.app, ConfirmButton::Send);
    assert_eq!(
        tasks.host.controls(),
        [Control::Signal(key_of(4242), Signal::Hangup)]
    );
}

#[test]
fn a_press_with_no_confirmation_open_does_nothing() {
    let mut tasks = firefox();
    press(&mut tasks.app, ConfirmButton::Terminate);
    tasks.app.close_overlay();
    press(&mut tasks.app, ConfirmButton::Terminate);
    assert!(tasks.host.controls().is_empty());
}

#[test]
fn the_frozen_targets_are_signalled_after_a_refresh_moved_every_row() {
    let mut tasks = TaskApp::open(vec![named(10, "vim"), named(20, "top")], Privilege::User);
    confirm_terminate(tasks.list());
    tasks.resample(vec![named(5, "cc"), named(20, "top")]);
    assert_eq!(tasks.shown(), [5, 20], "the cursor row is now cc");
    press(&mut tasks.app, ConfirmButton::Terminate);
    assert_eq!(
        tasks.host.controls(),
        [Control::Signal(key_of(10), Signal::Terminate)]
    );
}

#[test]
fn marked_processes_are_signalled_in_view_order() {
    let mut tasks = TaskApp::open(
        vec![named(30, "c"), named(10, "a"), named(20, "b")],
        Privilege::User,
    );
    tasks.list().invert_marks();
    confirm_terminate(tasks.list());
    press(&mut tasks.app, ConfirmButton::Terminate);
    let signalled: Vec<Control> = [10, 20, 30]
        .into_iter()
        .map(|raw| Control::Signal(key_of(raw), Signal::Terminate))
        .collect();
    assert_eq!(tasks.host.controls(), signalled);
}

#[test]
fn ffm_itself_and_init_get_a_notice_and_no_call() {
    for (raw, refusal) in [(OWN, ControlRefusal::OwnProcess), (1, ControlRefusal::Init)] {
        let mut tasks = TaskApp::open(vec![named(raw, "self")], Privilege::Root);
        confirm_terminate(tasks.list());
        assert_eq!(tasks.list().modal(), &Modal::Browse);
        assert_eq!(tasks.list().notice(), Some(Notice::Refused(refusal)));
        open_signal_menu(tasks.list());
        assert_eq!(tasks.list().modal(), &Modal::Browse);
        assert!(tasks.host.controls().is_empty());
    }
}

#[test]
fn a_mixed_set_confirms_the_rest_and_lists_the_refused() {
    let mut tasks = TaskApp::open(
        vec![named(OWN, "ffm"), named(4242, "firefox")],
        Privilege::User,
    );
    tasks.list().invert_marks();
    confirm_terminate(tasks.list());
    let Modal::Confirm(confirm) = tasks.list().modal() else {
        panic!("the confirmation is open");
    };
    assert_eq!(confirm.targets().targets().len(), 1);
    assert_eq!(
        confirm.targets().skipped()[0].refusal(),
        ControlRefusal::OwnProcess
    );
    press(&mut tasks.app, ConfirmButton::Terminate);
    assert_eq!(
        tasks.host.controls(),
        [Control::Signal(key_of(4242), Signal::Terminate)]
    );
}

#[cfg(unix)]
#[test]
fn the_signal_menu_opens_on_sigterm_for_the_frozen_targets() {
    let mut tasks = firefox();
    open_signal_menu(tasks.list());
    let Modal::Menu(menu @ crate::models::TaskMenu::Signal { targets, .. }) = tasks.list().modal()
    else {
        panic!("the signal menu is open");
    };
    assert_eq!(menu.cursor().index(), 0);
    assert_eq!(targets.targets()[0].key(), key_of(4242));
}

#[test]
fn a_failed_signal_opens_the_error_dialog_with_one_line_per_target() {
    let mut tasks = TaskApp::open(
        vec![named(10, "a"), named(20, "b"), named(30, "c")],
        Privilege::User,
    );
    tasks.host.fail(key_of(10), ControlError::Gone);
    tasks.host.fail(
        key_of(30),
        ControlError::Failed(std::io::ErrorKind::Interrupted),
    );
    tasks.list().invert_marks();
    confirm_terminate(tasks.list());
    press(&mut tasks.app, ConfirmButton::Terminate);
    let Modal::Error(failures) = tasks.list().modal() else {
        panic!("the error dialog is open");
    };
    let lines: Vec<String> = failures.iter().map(ToString::to_string).collect();
    assert_eq!(
        lines,
        [
            "10 a has already exited; nothing was sent.",
            "Could not signal 30 c: operation interrupted.",
        ]
    );
    assert_eq!(tasks.host.controls().len(), 3, "every target was tried");
    assert_eq!(tasks.list().notice(), None);
    assert_eq!(tasks.list().refresh().schedule(), Schedule::Due);
}

#[test]
fn a_list_without_a_host_controls_nothing() {
    let mut tasks = firefox();
    tasks.app.task_host = crate::models::TaskHostSlot::default();
    confirm_terminate(tasks.list());
    press(&mut tasks.app, ConfirmButton::Terminate);
    let Modal::Error(failures) = tasks.list().modal() else {
        panic!("the error dialog is open");
    };
    assert_eq!(failures[0].error(), ControlError::Unsupported);
    step_nice(&mut tasks.app, NiceStep::Raise);
    assert!(tasks.host.controls().is_empty());
}

/// Windows has no signals to choose from, so F9 asks what F8 asks.
#[cfg(not(unix))]
#[test]
fn without_signals_to_offer_the_signal_menu_is_the_terminate_confirmation() {
    let mut tasks = firefox();
    open_signal_menu(tasks.list());
    let Modal::Confirm(confirm) = tasks.list().modal() else {
        panic!("the confirmation is open");
    };
    assert_eq!(confirm.offer(), KillOffer::TerminateOrKill);
}
