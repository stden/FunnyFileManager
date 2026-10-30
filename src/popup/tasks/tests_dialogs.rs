//! Purpose: Tests for the task list's dialogs through their keys: F8 and a
//! button, the letters and the focus of the confirmation, the Priority form,
//! `[`'s confirmation, the mask dialog, the help page and the dialogs that
//! only close.

use crossterm::event::{KeyCode, KeyModifiers};

#[cfg(unix)]
use crate::models::Signal;
use crate::models::{
    ControlError, ControlRefusal, KernelPriority, Modal, Notice, Priority, PriorityChange,
    ProcessSample,
};
use crate::prompt::Privilege;
use crate::test_support::processes::Control;
use crate::test_support::task_list::{key_of, named, nice, TaskApp, OWN};

use super::tests::{chord, press, three, type_text};

mod tests_mask_and_help;
mod tests_priority;

fn firefox(privilege: Privilege) -> TaskApp {
    let at_five = ProcessSample {
        priority: Priority::Unix {
            kernel: KernelPriority::new(25),
            nice: nice(5),
        },
        ..named(4242, "firefox")
    };
    TaskApp::open(vec![at_five], privilege)
}

#[cfg(unix)]
fn signalled(signal: Signal) -> Vec<Control> {
    vec![Control::Signal(key_of(4242), signal)]
}

fn reniced(value: i32) -> Vec<Control> {
    vec![Control::Priority(
        key_of(4242),
        PriorityChange::Nice(nice(value)),
    )]
}

#[cfg(unix)]
#[test]
fn f8_then_enter_terminates_and_tab_then_enter_kills() {
    let mut tasks = firefox(Privilege::User);
    press(&mut tasks, KeyCode::F(8));
    press(&mut tasks, KeyCode::Enter);
    assert_eq!(tasks.host.controls(), signalled(Signal::Terminate));
    let mut tasks = firefox(Privilege::User);
    press(&mut tasks, KeyCode::Delete);
    press(&mut tasks, KeyCode::Tab);
    press(&mut tasks, KeyCode::Enter);
    assert_eq!(tasks.host.controls(), signalled(Signal::Kill));
}

#[test]
fn the_confirmation_cancels_on_esc_f10_c_and_its_cancel_button() {
    let cancels = [
        vec![KeyCode::Esc],
        vec![KeyCode::F(10)],
        vec![KeyCode::Char('c')],
        vec![KeyCode::Left, KeyCode::Enter],
        vec![KeyCode::BackTab, KeyCode::Enter],
    ];
    for keys in cancels {
        let mut tasks = firefox(Privilege::User);
        press(&mut tasks, KeyCode::F(8));
        for code in &keys {
            press(&mut tasks, *code);
        }
        assert_eq!(tasks.host.controls().len(), 0, "{keys:?}");
        assert_eq!(tasks.list().modal(), &Modal::Browse, "{keys:?}");
    }
}

#[cfg(unix)]
#[test]
fn the_confirmation_letters_press_their_buttons() {
    let mut tasks = firefox(Privilege::User);
    press(&mut tasks, KeyCode::F(8));
    press(&mut tasks, KeyCode::Char('k'));
    assert_eq!(tasks.host.controls(), signalled(Signal::Kill));
    let mut tasks = firefox(Privilege::User);
    press(&mut tasks, KeyCode::F(8));
    press(&mut tasks, KeyCode::BackTab);
    press(&mut tasks, KeyCode::Char('x'));
    press(&mut tasks, KeyCode::Char('t'));
    assert_eq!(tasks.host.controls(), signalled(Signal::Terminate));
}

#[test]
fn a_refused_target_is_a_notice_and_no_dialog() {
    let mut tasks = TaskApp::open(vec![named(OWN, "ffm")], Privilege::Root);
    press(&mut tasks, KeyCode::F(8));
    assert_eq!(tasks.list().modal(), &Modal::Browse);
    assert_eq!(
        tasks.list().notice(),
        Some(Notice::Refused(ControlRefusal::OwnProcess))
    );
    assert_eq!(tasks.host.controls().len(), 0);
}

#[test]
fn the_error_dialog_lists_the_failure_and_closes_on_enter_or_f3() {
    for close in [KeyCode::Enter, KeyCode::F(3)] {
        let mut tasks = firefox(Privilege::User);
        tasks.host.fail(key_of(4242), ControlError::Gone);
        press(&mut tasks, KeyCode::F(8));
        press(&mut tasks, KeyCode::Enter);
        let Modal::Error { failures, .. } = tasks.list().modal() else {
            panic!("the error dialog is open");
        };
        assert_eq!(
            failures[0].to_string(),
            "4242 firefox has already exited; nothing was sent."
        );
        press(&mut tasks, KeyCode::Char('x'));
        assert!(matches!(tasks.list().modal(), Modal::Error { .. }));
        press(&mut tasks, close);
        assert_eq!(tasks.list().modal(), &Modal::Browse);
    }
}
