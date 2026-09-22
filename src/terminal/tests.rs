use super::*;
use crate::WritableTestBackend;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone)]
struct RecordingOps {
    calls: Rc<RefCell<Vec<&'static str>>>,
    fail_on: Option<&'static str>,
}

impl RecordingOps {
    fn new(fail_on: Option<&'static str>) -> Self {
        Self {
            calls: Rc::new(RefCell::new(Vec::new())),
            fail_on,
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
            "disable_mouse",
            "leave_alternate",
            "disable_raw",
            "show_cursor",
        ]
    );
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
            "disable_mouse",
            "leave_alternate",
            "disable_raw",
            "enable_raw",
            "enter_alternate",
            "enable_mouse",
            "clear",
        ]
    );
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
            "disable_mouse",
            "leave_alternate",
            "enter_alternate",
            "enable_mouse",
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

    assert!(calls
        .borrow()
        .windows(4)
        .any(|window| window == ["enable_raw", "enter_alternate", "enable_mouse", "clear"]));
}
