//! Purpose: Terminate and Kill reach the frozen targets once, in view order.

use super::*;

#[test]
fn f8_then_terminate_signals_the_cursor_process_once() {
    let mut tasks = firefox();
    confirm_terminate(tasks.list());
    let Modal::Confirm(confirm) = tasks.list().modal() else {
        panic!("the confirmation is open");
    };
    assert_eq!(confirm.offer(), KillOffer::TerminateOrKill);
    assert_eq!(tasks.host.controls().len(), 0, "opening sends nothing");
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
    assert_eq!(tasks.host.controls().len(), 0);
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
    assert_eq!(tasks.host.controls().len(), 0);
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
    assert_eq!(tasks.host.controls().len(), 0);
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
