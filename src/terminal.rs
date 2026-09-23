//! Purpose: Owns injected terminal operations and RAII lifecycle guards.

use crossterm::{
    cursor::Show,
    event::{DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
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

pub struct CrosstermOps;

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

    fn enable_keyboard(&mut self, terminal: &mut Terminal<B>) -> io::Result<KeyboardProtocol> {
        keyboard::push(terminal.backend_mut(), keyboard::host_protocol())
    }

    fn disable_keyboard(&mut self, terminal: &mut Terminal<B>) -> io::Result<KeyboardProtocol> {
        keyboard::pop(terminal.backend_mut(), keyboard::known_protocol())
    }

    fn wait_for_key(&mut self, terminal: &mut Terminal<B>, prompt: &str) -> io::Result<()> {
        let screen = terminal.backend_mut();
        // Raw mode: a bare `\n` would not return the cursor to column 0.
        write!(screen, "\r\n{prompt}")?;
        Write::flush(screen)?;
        // Read the way the event loop reads, which survives a hung-up
        // terminal; live and unrecorded, since the key only ends the pause.
        let mut input = crate::recording::Input::Live(None);
        while !input.read()?.as_ref().is_some_and(is_key_press) {}
        // End the prompt's line, so whatever runs next starts on its own.
        write!(screen, "\r\n")?;
        Write::flush(screen)
    }
}

/// A key going down: its release, mouse moves and resizes do not end a pause.
fn is_key_press(event: &Event) -> bool {
    matches!(event, Event::Key(key) if key.kind == KeyEventKind::Press)
}

#[cfg(any(test, coverage))]
pub struct NoopTerminalOps;

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

    fn wait_for_key(&mut self, _terminal: &mut Terminal<B>, _prompt: &str) -> io::Result<()> {
        Ok(())
    }
}

mod guards;
mod interrupts;
mod keyboard;

pub use guards::{TerminalSession, TerminalSuspension};
pub use keyboard::{decode_input, KeyboardProtocol};

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
/// What Alt+F9 got from the host terminal: the height it asked for and the
/// height the terminal has once it had a moment to act on the request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoMode {
    pub requested_rows: u16,
    pub rows: u16,
}

/// How long a terminal gets to apply a resize before its size is read back,
/// and how often it is asked in the meantime. Terminals resize
/// asynchronously; many (tmux among them) ignore the request altogether.
const RESIZE_WAIT: std::time::Duration = std::time::Duration::from_millis(300);
const RESIZE_POLL: std::time::Duration = std::time::Duration::from_millis(20);

/// Ask the host terminal for 25 rows when it is taller than 30, else for 50,
/// keeping its width, and report the height it then has.
pub fn toggle_video_mode() -> io::Result<VideoMode> {
    #[cfg(test)]
    {
        VIDEO_MODE_CALLS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if SKIP_VIDEO_MODE_RESIZE.load(std::sync::atomic::Ordering::SeqCst) {
            // A terminal that ignores the request, as tmux does.
            return Ok(VideoMode {
                requested_rows: 25,
                rows: 40,
            });
        }
    }
    let (width, height) = crossterm::terminal::size()?;
    let requested_rows = if height > 30 { 25 } else { 50 };
    execute!(
        io::stdout(),
        crossterm::terminal::SetSize(width, requested_rows)
    )?;
    Ok(VideoMode {
        requested_rows,
        rows: settled_rows(height),
    })
}

/// The terminal's height once it changed from `before`, or `before` when it
/// did not change within [`RESIZE_WAIT`].
fn settled_rows(before: u16) -> u16 {
    let deadline = std::time::Instant::now() + RESIZE_WAIT;
    loop {
        let rows = crossterm::terminal::size().map_or(before, |(_, rows)| rows);
        if rows != before || std::time::Instant::now() >= deadline {
            return rows;
        }
        std::thread::sleep(RESIZE_POLL);
    }
}

#[cfg(test)]
static VIDEO_MODE_CALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

#[cfg(test)]
static SKIP_VIDEO_MODE_RESIZE: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

#[cfg(test)]
pub fn video_mode_calls() -> usize {
    VIDEO_MODE_CALLS.load(std::sync::atomic::Ordering::SeqCst)
}

/// Serialises the tests that skip the resize: one guard dropping must not
/// let another test's Alt+F9 resize the runner's terminal, or add to the
/// call count it is checking.
#[cfg(test)]
static VIDEO_MODE_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Skip the host resize until the guard drops. Tests use this so Alt+F9 can
/// prove it called [`toggle_video_mode`] without changing the terminal.
#[cfg(test)]
pub fn skip_video_mode_resize() -> VideoModeGuard {
    let lock = VIDEO_MODE_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    SKIP_VIDEO_MODE_RESIZE.store(true, std::sync::atomic::Ordering::SeqCst);
    VideoModeGuard { _lock: lock }
}

#[cfg(test)]
pub struct VideoModeGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
}

#[cfg(test)]
impl Drop for VideoModeGuard {
    fn drop(&mut self) {
        SKIP_VIDEO_MODE_RESIZE.store(false, std::sync::atomic::Ordering::SeqCst);
    }
}
