//! Purpose: `TerminalSession` — enter raw mode, the alternate screen, mouse capture and the keyboard protocol, and undo them in reverse on drop.

use ratatui::{backend::Backend, Terminal};
use std::io::{self, Write};

use super::{finish_cleanup, retain_first_error};
use crate::terminal::{KeyboardProtocol, TerminalOps};

#[expect(
    clippy::struct_excessive_bools,
    reason = "one flag per terminal mode still to undo; restore retries each alone, so every combination occurs"
)]
pub struct TerminalSession<'a, B, O>
where
    B: Backend + Write,
    O: TerminalOps<B>,
{
    terminal: &'a mut Terminal<B>,
    ops: &'a mut O,
    raw: bool,
    alternate: bool,
    mouse: bool,
    /// The protocol this session pushed; `Legacy` again once it is popped.
    keyboard: KeyboardProtocol,
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
            keyboard: KeyboardProtocol::Legacy,
            cursor_pending: true,
        };

        session.raw = true;
        session.ops.enable_raw()?;
        session.alternate = true;
        session.ops.enter_alternate(session.terminal)?;
        session.mouse = true;
        session.ops.enable_mouse(session.terminal)?;
        session.keyboard = session.ops.enable_keyboard(session.terminal)?;
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

        // First, while the alternate screen is still up: each screen keeps
        // its own stack of keyboard flags.
        if self.keyboard == KeyboardProtocol::Kitty
            && retain_first_error(
                &mut first_error,
                self.ops.disable_keyboard(self.terminal).map(drop),
            )
        {
            self.keyboard = KeyboardProtocol::Legacy;
        }
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
        // A destructor cannot report; `finish` is the path that returns it.
        drop(self.restore());
    }
}
