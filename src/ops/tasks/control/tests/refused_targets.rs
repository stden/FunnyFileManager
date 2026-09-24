//! Purpose: Refused targets, the signal menu, failures and a list without a host.

use super::*;

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
