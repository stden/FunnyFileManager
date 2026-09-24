//! Purpose: Tests for the task list's dialogs through their keys: F8 and a
//! button, the letters and the focus of the confirmation, the Priority form,
//! `[`'s confirmation, the mask dialog, the help page and the dialogs that
//! only close.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::action::tasks::HELP;
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
        assert!(tasks.host.controls().is_empty(), "{keys:?}");
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
    assert!(tasks.host.controls().is_empty());
}

#[test]
fn the_error_dialog_lists_the_failure_and_closes_on_enter_or_f3() {
    for close in [KeyCode::Enter, KeyCode::F(3)] {
        let mut tasks = firefox(Privilege::User);
        tasks.host.fail(key_of(4242), ControlError::Gone);
        press(&mut tasks, KeyCode::F(8));
        press(&mut tasks, KeyCode::Enter);
        let Modal::Error(failures) = tasks.list().modal() else {
            panic!("the error dialog is open");
        };
        assert_eq!(
            failures[0].to_string(),
            "4242 firefox has already exited; nothing was sent."
        );
        press(&mut tasks, KeyCode::Char('x'));
        assert!(matches!(tasks.list().modal(), Modal::Error(_)));
        press(&mut tasks, close);
        assert_eq!(tasks.list().modal(), &Modal::Browse);
    }
}

#[test]
fn the_priority_form_steps_within_the_range_and_sets_on_enter() {
    let mut tasks = firefox(Privilege::User);
    press(&mut tasks, KeyCode::F(7));
    for _ in 0..30 {
        press(&mut tasks, KeyCode::Up);
    }
    press(&mut tasks, KeyCode::Down);
    press(&mut tasks, KeyCode::Enter);
    assert_eq!(tasks.host.controls(), reniced(-19));
    assert_eq!(tasks.list().modal(), &Modal::Browse);
}

#[test]
fn the_priority_form_edits_its_field_and_set_waits_for_a_nice_value() {
    let mut tasks = firefox(Privilege::User);
    press(&mut tasks, KeyCode::F(7));
    press(&mut tasks, KeyCode::Backspace);
    type_text(&mut tasks, "33");
    press(&mut tasks, KeyCode::Enter);
    assert!(
        matches!(tasks.list().modal(), Modal::Priority(_)),
        "disabled"
    );
    press(&mut tasks, KeyCode::Backspace);
    press(&mut tasks, KeyCode::Tab);
    press(&mut tasks, KeyCode::Char('9'));
    press(&mut tasks, KeyCode::Enter);
    assert_eq!(tasks.host.controls(), reniced(3), "Set, not typed into");
}

#[test]
fn the_priority_forms_cancel_and_escape_set_nothing() {
    for keys in [
        vec![KeyCode::Tab, KeyCode::Tab, KeyCode::Enter],
        vec![KeyCode::BackTab, KeyCode::Enter],
        vec![KeyCode::F(10)],
        vec![KeyCode::Esc],
    ] {
        let mut tasks = firefox(Privilege::User);
        press(&mut tasks, KeyCode::F(7));
        for code in &keys {
            press(&mut tasks, *code);
        }
        assert!(tasks.host.controls().is_empty(), "{keys:?}");
        assert_eq!(tasks.list().modal(), &Modal::Browse, "{keys:?}");
    }
}

#[test]
fn square_brackets_step_the_nice_value_and_a_user_confirms_lowering() {
    let mut tasks = firefox(Privilege::User);
    press(&mut tasks, KeyCode::Char(']'));
    press(&mut tasks, KeyCode::Char(']'));
    assert_eq!(
        tasks.host.controls(),
        [reniced(4), reniced(3)].concat(),
        "the second step starts from the accepted value"
    );
    press(&mut tasks, KeyCode::Char('['));
    assert!(matches!(tasks.list().modal(), Modal::Renice(_)));
    press(&mut tasks, KeyCode::Tab);
    press(&mut tasks, KeyCode::Enter);
    assert_eq!(tasks.host.controls().len(), 2, "Cancel was focused");
    press(&mut tasks, KeyCode::Char('['));
    press(&mut tasks, KeyCode::Char('x'));
    press(&mut tasks, KeyCode::Enter);
    assert_eq!(tasks.host.controls().last(), reniced(4).last());
    press(&mut tasks, KeyCode::Char('['));
    press(&mut tasks, KeyCode::Esc);
    assert_eq!(tasks.host.controls().len(), 3);
}

#[test]
fn root_lowers_the_priority_at_once() {
    let mut tasks = firefox(Privilege::Root);
    press(&mut tasks, KeyCode::Char('['));
    assert_eq!(tasks.list().modal(), &Modal::Browse);
    assert_eq!(tasks.host.controls(), reniced(6));
}

#[test]
fn the_mask_dialog_marks_the_matching_names_and_says_how_many() {
    let mut tasks = three();
    press(&mut tasks, KeyCode::Char('+'));
    type_text(&mut tasks, "*n*");
    press(&mut tasks, KeyCode::Enter);
    assert_eq!(tasks.list().modal(), &Modal::Browse);
    assert_eq!(
        tasks.list().notice(),
        Some(Notice::Marked {
            changed: 2,
            total: 2
        })
    );
    press(&mut tasks, KeyCode::Char('-'));
    press(&mut tasks, KeyCode::Esc);
    assert_eq!(tasks.list().marked_count(), 2, "cancelled");
}

#[test]
fn the_help_page_scrolls_within_its_rows_and_closes_on_its_keys() {
    let mut tasks = three();
    let viewport = crate::models::Viewport::new(12);
    tasks.app.viewport = viewport;
    let page = tasks.list().page_for(viewport);
    let last = HELP.len() - page;
    let top = |tasks: &mut TaskApp| match tasks.list().modal() {
        Modal::Help(top) => top.lines(),
        other => panic!("the help page is open, not {other:?}"),
    };
    press(&mut tasks, KeyCode::F(1));
    press(&mut tasks, KeyCode::End);
    assert_eq!(top(&mut tasks), last);
    press(&mut tasks, KeyCode::Down);
    assert_eq!(top(&mut tasks), last, "no further");
    press(&mut tasks, KeyCode::PageUp);
    press(&mut tasks, KeyCode::Up);
    assert_eq!(top(&mut tasks), last - page - 1);
    press(&mut tasks, KeyCode::PageDown);
    press(&mut tasks, KeyCode::Home);
    assert_eq!(top(&mut tasks), 0);
    press(&mut tasks, KeyCode::Char('x'));
    assert_eq!(top(&mut tasks), 0);
}

#[test]
fn the_help_page_closes_on_its_keys_and_only_the_page_closes() {
    let mut tasks = three();
    for close in ['q', 'h', '?'] {
        press(&mut tasks, KeyCode::F(1));
        press(&mut tasks, KeyCode::Char(close));
        assert_eq!(tasks.list().modal(), &Modal::Browse, "{close}");
    }
    chord(&mut tasks, KeyCode::Char('?'), KeyModifiers::SHIFT);
    press(&mut tasks, KeyCode::F(1));
    assert_eq!(tasks.list().modal(), &Modal::Browse);
    press(&mut tasks, KeyCode::Char('h'));
    press(&mut tasks, KeyCode::F(10));
    assert_eq!(tasks.list().modal(), &Modal::Browse);
    assert!(tasks.app.overlay.is_active(), "F10 closed only the help");
}
