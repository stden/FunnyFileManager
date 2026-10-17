//! Purpose: Tests for the Send signal menu, where signals exist: choosing a
//! signal only opens its confirmation, for the targets the menu froze.

use crossterm::event::KeyCode;

use crate::models::{KillOffer, Modal, Signal};
use crate::prompt::Privilege;
use crate::test_support::processes::Control;
use crate::test_support::task_list::{key_of, named, TaskApp};

use super::tests::press;

#[test]
fn f9_down_to_sighup_then_enter_and_enter_sends_sighup() {
    let mut tasks = TaskApp::open(vec![named(4242, "firefox")], Privilege::User);
    press(&mut tasks, KeyCode::F(9));
    press(&mut tasks, KeyCode::Down);
    press(&mut tasks, KeyCode::Down);
    press(&mut tasks, KeyCode::Enter);
    let Modal::Confirm(confirm) = tasks.list().modal() else {
        panic!("the confirmation is open");
    };
    assert_eq!(confirm.offer(), KillOffer::Send(Signal::Hangup));
    assert!(tasks.host.controls().is_empty(), "the menu sends nothing");
    press(&mut tasks, KeyCode::Enter);
    assert_eq!(
        tasks.host.controls(),
        [Control::Signal(key_of(4242), Signal::Hangup)]
    );
}

#[test]
fn the_signal_menu_keeps_its_targets_through_a_refresh() {
    let mut tasks = TaskApp::open(vec![named(10, "a"), named(20, "b")], Privilege::User);
    press(&mut tasks, KeyCode::Char('k'));
    tasks.resample(vec![named(5, "c"), named(20, "b")]);
    press(&mut tasks, KeyCode::Enter);
    press(&mut tasks, KeyCode::Char('s'));
    assert_eq!(
        tasks.host.controls(),
        [Control::Signal(key_of(10), Signal::Terminate)]
    );
}

/// A Ctrl chord is a command, never a button's letter: Ctrl+K in the
/// confirmation must not send SIGKILL.
#[test]
fn a_ctrl_chord_in_the_confirmation_presses_no_button() {
    let mut tasks = TaskApp::open(vec![named(4242, "firefox")], Privilege::User);
    press(&mut tasks, KeyCode::F(8));
    for letter in ['k', 't', 'K'] {
        super::tests::chord(
            &mut tasks,
            KeyCode::Char(letter),
            crossterm::event::KeyModifiers::CONTROL,
        );
    }
    assert!(tasks.host.controls().is_empty());
    assert!(
        matches!(tasks.list().modal(), Modal::Confirm(_)),
        "still open"
    );
    press(&mut tasks, KeyCode::Right);
    press(&mut tasks, KeyCode::Enter);
    assert_eq!(
        tasks.host.controls(),
        [Control::Signal(key_of(4242), Signal::Kill)],
        "Right moves the focus to Kill"
    );
}
