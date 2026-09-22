//! Purpose: RAII guards that enter and restore the terminal session, and suspend it around an external program.

use ratatui::{backend::Backend, Terminal};
use std::io::{self, Write};

use super::TerminalOps;

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
