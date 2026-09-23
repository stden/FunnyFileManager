//! Purpose: RAII guards that enter and restore the terminal session, and suspend it around an external program.

use ratatui::{backend::Backend, Terminal};
use std::io::{self, Write};

use super::interrupts::InterruptShield;
use super::{KeyboardProtocol, TerminalOps};

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

#[expect(
    clippy::struct_excessive_bools,
    reason = "one flag per terminal mode still to undo; restore retries each alone, so every combination occurs"
)]
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
        let _ = self.restore();
    }
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "one flag per terminal mode still to resume; each is retried alone, so every combination occurs"
)]
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
    /// The protocol `suspend` popped and `resume` has yet to push again.
    keyboard_popped: KeyboardProtocol,
    clear_pending: bool,
    /// Up for the whole hand-off: with raw mode off the terminal turns Ctrl+C
    /// into a signal for FFM's process group. A field, so it is lowered only
    /// after `Drop` has put raw mode back.
    interrupts: InterruptShield,
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
            keyboard_popped: KeyboardProtocol::Legacy,
            clear_pending: false,
            interrupts: InterruptShield::raise()?,
        };

        // The program that runs next reads keys its own way.
        suspension.keyboard_popped = suspension.ops.disable_keyboard(suspension.terminal)?;
        suspension.mouse_disabled = true;
        suspension.clear_pending = true;
        suspension.ops.disable_mouse(suspension.terminal)?;
        suspension.alternate_left = true;
        suspension.ops.leave_alternate(suspension.terminal)?;
        suspension.raw_disabled = true;
        suspension.ops.disable_raw()?;
        Ok(suspension)
    }

    /// Whether Ctrl+C or Ctrl+\ was pressed while the program had the
    /// terminal. The program got the signal; FFM only reports it.
    pub(crate) fn interrupted(&self) -> bool {
        self.interrupts.interrupted()
    }

    pub(crate) fn resume(mut self) -> io::Result<()> {
        self.resume_all()
    }

    /// [`Self::resume`], once the user has read what the program printed.
    ///
    /// Far leaves a program's output on its console screen; FFM's panels would
    /// cover it the moment they are back. So `prompt` goes under the output
    /// and the panels return on the next key. Raw mode comes back first, so
    /// that one key, not a whole line, answers; without it there is no pause.
    pub(crate) fn resume_after_key(mut self, prompt: &str) -> io::Result<()> {
        let mut first_error = None;
        if retain_first_error(&mut first_error, self.ops.enable_raw()) {
            self.raw_disabled = false;
            retain_first_error(
                &mut first_error,
                self.ops.wait_for_key(self.terminal, prompt),
            );
        }
        let rest = self.resume_all();
        finish_cleanup(first_error).and(rest)
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
        if self.keyboard_popped == KeyboardProtocol::Kitty
            && retain_first_error(
                &mut first_error,
                self.ops.enable_keyboard(self.terminal).map(drop),
            )
        {
            self.keyboard_popped = KeyboardProtocol::Legacy;
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
