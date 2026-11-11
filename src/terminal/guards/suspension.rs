//! Purpose: `TerminalSuspension` — hand the terminal to an external program and take it back in forward order on resume or drop.

use ratatui::{backend::Backend, Terminal};
use std::io::{self, Write};

use super::{finish_cleanup, retain_first_error};
use crate::terminal::interrupts::InterruptShield;
use crate::terminal::{KeyboardProtocol, TerminalOps};

#[expect(
    clippy::struct_excessive_bools,
    reason = "one flag per terminal mode still to resume; each is retried alone, so every combination occurs"
)]
pub struct TerminalSuspension<'a, B, O>
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
    _perf_pause: crate::performance::PauseGuard,
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
            _perf_pause: crate::performance::pause(),
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
        // A destructor cannot report; `resume` is the path that returns it.
        drop(self.resume_all());
    }
}
