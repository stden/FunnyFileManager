use super::*;
use crate::WritableTestBackend;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone)]
struct RecordingOps {
    calls: Rc<RefCell<Vec<&'static str>>>,
    fail_on: Option<&'static str>,
    /// What the pretend host supports: `Kitty` unless a test says otherwise.
    keyboard: KeyboardProtocol,
}

impl RecordingOps {
    fn new(fail_on: Option<&'static str>) -> Self {
        Self {
            calls: Rc::new(RefCell::new(Vec::new())),
            fail_on,
            keyboard: KeyboardProtocol::Kitty,
        }
    }

    fn legacy() -> Self {
        Self {
            keyboard: KeyboardProtocol::Legacy,
            ..Self::new(None)
        }
    }

    fn record(&self, operation: &'static str) -> io::Result<()> {
        self.calls.borrow_mut().push(operation);
        if self.fail_on == Some(operation) {
            Err(io::Error::other(operation))
        } else {
            Ok(())
        }
    }
}

impl TerminalOps<WritableTestBackend> for RecordingOps {
    fn enable_raw(&mut self) -> io::Result<()> {
        self.record("enable_raw")
    }

    fn disable_raw(&mut self) -> io::Result<()> {
        self.record("disable_raw")
    }

    fn enter_alternate(&mut self, _terminal: &mut Terminal<WritableTestBackend>) -> io::Result<()> {
        self.record("enter_alternate")
    }

    fn leave_alternate(&mut self, _terminal: &mut Terminal<WritableTestBackend>) -> io::Result<()> {
        self.record("leave_alternate")
    }

    fn enable_mouse(&mut self, _terminal: &mut Terminal<WritableTestBackend>) -> io::Result<()> {
        self.record("enable_mouse")
    }

    fn disable_mouse(&mut self, _terminal: &mut Terminal<WritableTestBackend>) -> io::Result<()> {
        self.record("disable_mouse")
    }

    fn clear(&mut self, _terminal: &mut Terminal<WritableTestBackend>) -> io::Result<()> {
        self.record("clear")
    }

    fn show_cursor(&mut self, _terminal: &mut Terminal<WritableTestBackend>) -> io::Result<()> {
        self.record("show_cursor")
    }

    fn enable_keyboard(
        &mut self,
        _terminal: &mut Terminal<WritableTestBackend>,
    ) -> io::Result<KeyboardProtocol> {
        self.record("enable_keyboard").map(|()| self.keyboard)
    }

    fn disable_keyboard(
        &mut self,
        _terminal: &mut Terminal<WritableTestBackend>,
    ) -> io::Result<KeyboardProtocol> {
        self.record("disable_keyboard").map(|()| self.keyboard)
    }

    fn wait_for_key(
        &mut self,
        _terminal: &mut Terminal<WritableTestBackend>,
        _prompt: &str,
    ) -> io::Result<()> {
        self.record("wait_for_key")
    }
}

fn terminal() -> Terminal<WritableTestBackend> {
    Terminal::new(WritableTestBackend::new(80, 24)).expect("terminal")
}

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
    let suspension = TerminalSuspension::suspend(&mut terminal, &mut ops).expect("suspend");
    suspension.resume_after_key("press a key").expect("resume");

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
