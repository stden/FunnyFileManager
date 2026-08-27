//! Purpose: Owns injected terminal operations and RAII lifecycle guards.
#![allow(clippy::redundant_pub_crate)]

use crossterm::{
    cursor::Show,
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::Backend, Terminal};
use std::io::{self, Write};

pub(crate) trait TerminalOps<B>
where
    B: Backend + Write,
{
    fn enable_raw(&mut self) -> io::Result<()>;
    fn disable_raw(&mut self) -> io::Result<()>;
    fn enter_alternate(&mut self, terminal: &mut Terminal<B>) -> io::Result<()>;
    fn leave_alternate(&mut self, terminal: &mut Terminal<B>) -> io::Result<()>;
    fn enable_mouse(&mut self, terminal: &mut Terminal<B>) -> io::Result<()>;
    fn disable_mouse(&mut self, terminal: &mut Terminal<B>) -> io::Result<()>;
    fn clear(&mut self, terminal: &mut Terminal<B>) -> io::Result<()>;
    fn show_cursor(&mut self, terminal: &mut Terminal<B>) -> io::Result<()>;
}

pub(crate) struct CrosstermOps;

impl<B> TerminalOps<B> for CrosstermOps
where
    B: Backend + Write,
{
    fn enable_raw(&mut self) -> io::Result<()> {
        enable_raw_mode()
    }

    fn disable_raw(&mut self) -> io::Result<()> {
        disable_raw_mode()
    }

    fn enter_alternate(&mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        execute!(terminal.backend_mut(), EnterAlternateScreen)
    }

    fn leave_alternate(&mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        execute!(terminal.backend_mut(), LeaveAlternateScreen)
    }

    fn enable_mouse(&mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        execute!(terminal.backend_mut(), EnableMouseCapture)
    }

    fn disable_mouse(&mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        execute!(terminal.backend_mut(), DisableMouseCapture)
    }

    fn clear(&mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        terminal
            .clear()
            .map_err(|error| io::Error::other(error.to_string()))
    }

    fn show_cursor(&mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        terminal
            .show_cursor()
            .map_err(|error| io::Error::other(error.to_string()))
    }
}

#[cfg(any(test, coverage))]
#[cfg_attr(coverage, allow(dead_code))]
pub(crate) struct NoopTerminalOps;

#[cfg(any(test, coverage))]
impl<B> TerminalOps<B> for NoopTerminalOps
where
    B: Backend + Write,
{
    fn enable_raw(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn disable_raw(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn enter_alternate(&mut self, _terminal: &mut Terminal<B>) -> io::Result<()> {
        Ok(())
    }

    fn leave_alternate(&mut self, _terminal: &mut Terminal<B>) -> io::Result<()> {
        Ok(())
    }

    fn enable_mouse(&mut self, _terminal: &mut Terminal<B>) -> io::Result<()> {
        Ok(())
    }

    fn disable_mouse(&mut self, _terminal: &mut Terminal<B>) -> io::Result<()> {
        Ok(())
    }

    fn clear(&mut self, _terminal: &mut Terminal<B>) -> io::Result<()> {
        Ok(())
    }

    fn show_cursor(&mut self, _terminal: &mut Terminal<B>) -> io::Result<()> {
        Ok(())
    }
}

fn retain_first_error(first_error: &mut Option<io::Error>, result: io::Result<()>) -> bool {
    match result {
        Ok(()) => true,
        Err(error) => {
            if first_error.is_none() {
                *first_error = Some(error);
            }
            false
        }
    }
}

fn finish_cleanup(first_error: Option<io::Error>) -> io::Result<()> {
    first_error.map_or(Ok(()), Err)
}

pub(crate) struct TerminalSession<'a, B, O>
where
    B: Backend + Write,
    O: TerminalOps<B>,
{
    terminal: &'a mut Terminal<B>,
    ops: &'a mut O,
    raw: bool,
    alternate: bool,
    mouse: bool,
    cursor_pending: bool,
}

impl<'a, B, O> TerminalSession<'a, B, O>
where
    B: Backend + Write,
    O: TerminalOps<B>,
{
    pub(crate) fn enter(terminal: &'a mut Terminal<B>, ops: &'a mut O) -> io::Result<Self> {
        let mut session = Self {
            terminal,
            ops,
            raw: false,
            alternate: false,
            mouse: false,
            cursor_pending: true,
        };

        session.raw = true;
        session.ops.enable_raw()?;
        session.alternate = true;
        session.ops.enter_alternate(session.terminal)?;
        session.mouse = true;
        session.ops.enable_mouse(session.terminal)?;
        Ok(session)
    }

    pub(crate) const fn terminal_mut(&mut self) -> &mut Terminal<B> {
        self.terminal
    }

    pub(crate) fn finish(mut self) -> io::Result<()> {
        self.restore()
    }

    fn restore(&mut self) -> io::Result<()> {
        let mut first_error = None;

        if self.mouse && retain_first_error(&mut first_error, self.ops.disable_mouse(self.terminal))
        {
            self.mouse = false;
        }
        if self.alternate
            && retain_first_error(&mut first_error, self.ops.leave_alternate(self.terminal))
        {
            self.alternate = false;
        }
        if self.raw && retain_first_error(&mut first_error, self.ops.disable_raw()) {
            self.raw = false;
        }
        if self.cursor_pending
            && retain_first_error(&mut first_error, self.ops.show_cursor(self.terminal))
        {
            self.cursor_pending = false;
        }

        finish_cleanup(first_error)
    }
}

impl<B, O> Drop for TerminalSession<'_, B, O>
where
    B: Backend + Write,
    O: TerminalOps<B>,
{
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

pub(crate) struct TerminalSuspension<'a, B, O>
where
    B: Backend + Write,
    O: TerminalOps<B>,
{
    terminal: &'a mut Terminal<B>,
    ops: &'a mut O,
    raw_disabled: bool,
    alternate_left: bool,
    mouse_disabled: bool,
    clear_pending: bool,
}

impl<'a, B, O> TerminalSuspension<'a, B, O>
where
    B: Backend + Write,
    O: TerminalOps<B>,
{
    pub(crate) fn suspend(terminal: &'a mut Terminal<B>, ops: &'a mut O) -> io::Result<Self> {
        let mut suspension = Self {
            terminal,
            ops,
            raw_disabled: false,
            alternate_left: false,
            mouse_disabled: false,
            clear_pending: false,
        };

        suspension.mouse_disabled = true;
        suspension.clear_pending = true;
        suspension.ops.disable_mouse(suspension.terminal)?;
        suspension.alternate_left = true;
        suspension.ops.leave_alternate(suspension.terminal)?;
        suspension.raw_disabled = true;
        suspension.ops.disable_raw()?;
        Ok(suspension)
    }

    pub(crate) fn resume(mut self) -> io::Result<()> {
        self.resume_all()
    }

    fn resume_all(&mut self) -> io::Result<()> {
        let mut first_error = None;

        if self.raw_disabled && retain_first_error(&mut first_error, self.ops.enable_raw()) {
            self.raw_disabled = false;
        }
        if self.alternate_left
            && retain_first_error(&mut first_error, self.ops.enter_alternate(self.terminal))
        {
            self.alternate_left = false;
        }
        if self.mouse_disabled
            && retain_first_error(&mut first_error, self.ops.enable_mouse(self.terminal))
        {
            self.mouse_disabled = false;
        }
        if self.clear_pending && retain_first_error(&mut first_error, self.ops.clear(self.terminal))
        {
            self.clear_pending = false;
        }

        finish_cleanup(first_error)
    }
}

impl<B, O> Drop for TerminalSuspension<'_, B, O>
where
    B: Backend + Write,
    O: TerminalOps<B>,
{
    fn drop(&mut self) {
        let _ = self.resume_all();
    }
}

pub(crate) fn merge_run_and_cleanup<T>(
    run: io::Result<T>,
    cleanup: io::Result<()>,
) -> io::Result<T> {
    match (run, cleanup) {
        (Err(error), _) | (Ok(_), Err(error)) => Err(error),
        (Ok(value), Ok(())) => Ok(value),
    }
}

pub(crate) fn restore_after_abort() {
    let _ = disable_raw_mode();
    let _ = execute!(
        io::stdout(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        Show
    );
}

#[cfg(test)]
mod tests_crossterm_ops;

#[cfg(test)]
mod tests {
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

        fn enter_alternate(
            &mut self,
            _terminal: &mut Terminal<WritableTestBackend>,
        ) -> io::Result<()> {
            self.record("enter_alternate")
        }

        fn leave_alternate(
            &mut self,
            _terminal: &mut Terminal<WritableTestBackend>,
        ) -> io::Result<()> {
            self.record("leave_alternate")
        }

        fn enable_mouse(
            &mut self,
            _terminal: &mut Terminal<WritableTestBackend>,
        ) -> io::Result<()> {
            self.record("enable_mouse")
        }

        fn disable_mouse(
            &mut self,
            _terminal: &mut Terminal<WritableTestBackend>,
        ) -> io::Result<()> {
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
}
