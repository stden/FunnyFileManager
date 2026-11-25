//! Purpose: `TerminalSession` — entering the terminal step by step and restoring it in reverse, whatever fails.

use super::*;

#[test]
fn session_drop_restores_in_reverse_order() {
    let mut terminal = terminal();
    let mut ops = RecordingOps::new(None);
    let calls = Rc::clone(&ops.calls);
    {
        let _session = TerminalSession::enter(&mut terminal, &mut ops).expect("enter session");
    }

    assert_eq!(
        *calls.borrow(),
        [
            "enable_raw",
            "enter_alternate",
            "enable_mouse",
            "enable_keyboard",
            "disable_keyboard",
            "disable_mouse",
            "leave_alternate",
            "disable_raw",
            "show_cursor",
        ]
    );
}

/// A terminal that did not answer for the kitty protocol never had it
/// pushed, so it is never sent a pop either.
#[test]
fn a_legacy_terminal_session_pops_nothing() {
    let mut terminal = terminal();
    let mut ops = RecordingOps::legacy();
    let calls = Rc::clone(&ops.calls);
    let session = TerminalSession::enter(&mut terminal, &mut ops).expect("enter session");
    session.finish().expect("finish");

    assert!(
        !calls.borrow().contains(&"disable_keyboard"),
        "{:?}",
        calls.borrow()
    );
}

/// A push that failed pushed nothing: the rollback leaves the protocol alone
/// and still restores everything else.
#[test]
fn a_failed_keyboard_push_is_not_popped() {
    let mut terminal = terminal();
    let mut ops = RecordingOps::new(Some("enable_keyboard"));
    let calls = Rc::clone(&ops.calls);
    assert!(TerminalSession::enter(&mut terminal, &mut ops).is_err());

    assert_eq!(
        *calls.borrow(),
        [
            "enable_raw",
            "enter_alternate",
            "enable_mouse",
            "enable_keyboard",
            "disable_mouse",
            "leave_alternate",
            "disable_raw",
            "show_cursor",
        ]
    );
}

/// `finish` pops the flags once; the drop that follows must not pop again,
/// or it would take flags the shell pushed.
#[test]
fn a_finished_session_pops_the_protocol_once() {
    let mut terminal = terminal();
    let mut ops = RecordingOps::new(None);
    let calls = Rc::clone(&ops.calls);
    let session = TerminalSession::enter(&mut terminal, &mut ops).expect("enter session");
    session.finish().expect("finish");

    let pops = calls
        .borrow()
        .iter()
        .filter(|call| **call == "disable_keyboard")
        .count();
    assert_eq!(pops, 1);
}

#[test]
fn failed_enter_step_is_rolled_back() {
    let mut terminal = terminal();
    let mut ops = RecordingOps::new(Some("enter_alternate"));
    let calls = Rc::clone(&ops.calls);
    assert!(TerminalSession::enter(&mut terminal, &mut ops).is_err());

    assert_eq!(
        *calls.borrow(),
        [
            "enable_raw",
            "enter_alternate",
            "leave_alternate",
            "disable_raw",
            "show_cursor",
        ]
    );
}

#[test]
fn cleanup_continues_after_failure() {
    let mut terminal = terminal();
    let mut ops = RecordingOps::new(Some("disable_mouse"));
    let calls = Rc::clone(&ops.calls);
    let session = TerminalSession::enter(&mut terminal, &mut ops).expect("enter");
    assert!(session.finish().is_err());

    assert!(calls.borrow().windows(4).any(|window| window
        == [
            "disable_mouse",
            "leave_alternate",
            "disable_raw",
            "show_cursor",
        ]));
}

#[test]
fn run_error_has_priority_over_cleanup_error() {
    let run = Err::<(), _>(io::Error::other("run"));
    let cleanup = Err(io::Error::other("cleanup"));
    assert_eq!(
        merge_run_and_cleanup(run, cleanup)
            .expect_err("primary error")
            .to_string(),
        "run"
    );
}
