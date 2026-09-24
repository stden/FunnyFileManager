//! Purpose: Owns injected terminal operations and RAII lifecycle guards.

use crossterm::{
    cursor::Show, event::DisableMouseCapture, execute, terminal::disable_raw_mode,
    terminal::LeaveAlternateScreen,
};
use ratatui::{backend::Backend, Terminal};
use std::io::{self, Write};

pub trait TerminalOps<B>
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
    /// Push the kitty keyboard protocol where the terminal supports it; the
    /// answer is the protocol now in force. By default the terminal has no
    /// such protocol and nothing is written.
    fn enable_keyboard(&mut self, _terminal: &mut Terminal<B>) -> io::Result<KeyboardProtocol> {
        Ok(KeyboardProtocol::Legacy)
    }
    /// Pop what `enable_keyboard` pushed; the answer is the protocol to push
    /// again on resume.
    fn disable_keyboard(&mut self, _terminal: &mut Terminal<B>) -> io::Result<KeyboardProtocol> {
        Ok(KeyboardProtocol::Legacy)
    }
    /// Print `prompt` below what a program left on the screen and wait for
    /// one key press. Called with raw mode on, so any key answers.
    fn wait_for_key(&mut self, terminal: &mut Terminal<B>, prompt: &str) -> io::Result<()>;
}

mod crossterm_ops;
mod guards;
mod interrupts;
mod keyboard;
#[cfg(any(test, coverage))]
mod noop_ops;
mod video_mode;

pub use crossterm_ops::CrosstermOps;
pub use guards::{TerminalSession, TerminalSuspension};
pub use keyboard::{decode_input, KeyboardProtocol};
#[cfg(any(test, coverage))]
pub use noop_ops::NoopTerminalOps;
#[cfg(test)]
pub use video_mode::{skip_video_mode_resize, video_mode_calls};
pub use video_mode::{toggle_video_mode, VideoMode};

pub fn merge_run_and_cleanup<T>(run: io::Result<T>, cleanup: io::Result<()>) -> io::Result<T> {
    match (run, cleanup) {
        (Err(error), _) | (Ok(_), Err(error)) => Err(error),
        (Ok(value), Ok(())) => Ok(value),
    }
}

/// Best effort, after a panic: every step runs whatever the one before it
/// returned, and there is no caller left to hand an error to.
pub fn restore_after_abort() {
    drop(disable_raw_mode());
    let mut stdout = io::stdout();
    // Before the alternate screen goes: each screen has its own flag stack,
    // and a pop on the main screen would take the shell's flags instead.
    drop(keyboard::pop(&mut stdout, keyboard::known_protocol()));
    drop(execute!(
        stdout,
        LeaveAlternateScreen,
        DisableMouseCapture,
        Show
    ));
}

#[cfg(test)]
mod tests_crossterm_ops;

#[cfg(test)]
mod tests;
