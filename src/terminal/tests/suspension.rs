//! Purpose: `TerminalSuspension` — handing the terminal to a program and taking it back, with or without a pause.

use super::*;
use crate::terminal::crossterm_ops::is_key_press;

#[test]
fn suspension_resumes_in_forward_order() {
    let mut terminal = terminal();
    let mut ops = RecordingOps::new(None);
    let calls = Rc::clone(&ops.calls);
    let suspension = TerminalSuspension::suspend(&mut terminal, &mut ops).expect("suspend");
    suspension.resume().expect("resume");

    assert_eq!(
        *calls.borrow(),
        [
            "disable_keyboard",
            "disable_mouse",
            "leave_alternate",
            "disable_raw",
            "enable_raw",
            "enter_alternate",
            "enable_mouse",
            "enable_keyboard",
            "clear",
        ]
    );
}

/// An external program must get the keys its own way: the flags come off
/// before it runs. Where none were pushed, none are pushed on resume.
#[test]
fn a_legacy_suspension_pushes_nothing_on_resume() {
    let mut terminal = terminal();
    let mut ops = RecordingOps::legacy();
    let calls = Rc::clone(&ops.calls);
    let suspension = TerminalSuspension::suspend(&mut terminal, &mut ops).expect("suspend");
    suspension.resume().expect("resume");

    assert!(
        !calls.borrow().contains(&"enable_keyboard"),
        "{:?}",
        calls.borrow()
    );
}

/// Nobody pressed Ctrl+C: the hand-off ends without a cancel to report.
#[test]
fn a_hand_off_without_ctrl_c_is_not_interrupted() {
    let mut terminal = terminal();
    let mut ops = RecordingOps::new(None);
    let suspension = TerminalSuspension::suspend(&mut terminal, &mut ops).expect("suspend");
    assert!(!suspension.interrupted());
    suspension.resume().expect("resume");
}

#[test]
fn failed_suspend_step_is_resumed() {
    let mut terminal = terminal();
    let mut ops = RecordingOps::new(Some("leave_alternate"));
    let calls = Rc::clone(&ops.calls);
    assert!(TerminalSuspension::suspend(&mut terminal, &mut ops).is_err());

    assert_eq!(
        *calls.borrow(),
        [
            "disable_keyboard",
            "disable_mouse",
            "leave_alternate",
            "enter_alternate",
            "enable_mouse",
            "enable_keyboard",
            "clear",
        ]
    );
}

#[test]
fn suspension_resume_continues_after_failure() {
    let mut terminal = terminal();
    let mut ops = RecordingOps::new(Some("enter_alternate"));
    let calls = Rc::clone(&ops.calls);
    let suspension = TerminalSuspension::suspend(&mut terminal, &mut ops).expect("suspend");
    assert!(suspension.resume().is_err());

    assert!(calls.borrow().windows(5).any(|window| window
        == [
            "enable_raw",
            "enter_alternate",
            "enable_mouse",
            "enable_keyboard",
            "clear",
        ]));
}

/// The pause sits after raw mode is back, so one key answers, and before the
/// panels' screen returns, which would hide what the program printed.
#[test]
fn a_paused_resume_waits_for_a_key_before_the_panels_return() {
    let mut terminal = terminal();
    let mut ops = RecordingOps::new(None);
    let calls = Rc::clone(&ops.calls);
    let before = crate::performance::paused_total();
    let suspension = TerminalSuspension::suspend(&mut terminal, &mut ops).expect("suspend");
    std::thread::sleep(std::time::Duration::from_millis(20));
    suspension.resume_after_key("press a key").expect("resume");
    assert!(
        crate::performance::paused_total().saturating_sub(before)
            >= std::time::Duration::from_millis(15)
    );

    assert_eq!(
        *calls.borrow(),
        [
            "disable_keyboard",
            "disable_mouse",
            "leave_alternate",
            "disable_raw",
            "enable_raw",
            "wait_for_key",
            "enter_alternate",
            "enable_mouse",
            "enable_keyboard",
            "clear",
        ]
    );
}

/// Without raw mode a key press would need Enter; no pause then, and the
/// panels still come back.
#[test]
fn no_pause_without_raw_mode_and_the_panels_still_return() {
    let mut terminal = terminal();
    let mut ops = RecordingOps::new(Some("enable_raw"));
    let calls = Rc::clone(&ops.calls);
    let suspension = TerminalSuspension::suspend(&mut terminal, &mut ops).expect("suspend");
    let error = suspension
        .resume_after_key("press a key")
        .expect_err("raw mode failed");

    assert_eq!(error.to_string(), "enable_raw");
    assert!(!calls.borrow().contains(&"wait_for_key"));
    assert!(calls.borrow().contains(&"enter_alternate"));
}

#[test]
fn only_a_key_going_down_ends_a_pause() {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};
    let press = KeyEvent::from(KeyCode::Char('x'));
    let mut release = press;
    release.kind = KeyEventKind::Release;
    assert!(is_key_press(&Event::Key(press)));
    assert!(!is_key_press(&Event::Key(release)));
    assert!(!is_key_press(&Event::Resize(80, 24)));
}
