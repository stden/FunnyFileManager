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

mod guards;

pub(crate) use guards::{TerminalSession, TerminalSuspension};

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
mod tests;
/// Toggle between a compact and tall host terminal while retaining its width.
pub(crate) fn toggle_video_mode() -> std::io::Result<()> {
    #[cfg(test)]
    {
        VIDEO_MODE_CALLS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if SKIP_VIDEO_MODE_RESIZE.load(std::sync::atomic::Ordering::SeqCst) {
            return Ok(());
        }
    }
    let (width, height) = crossterm::terminal::size()?;
    crossterm::execute!(
        std::io::stdout(),
        crossterm::terminal::SetSize(width, if height > 30 { 25 } else { 50 })
    )
}

#[cfg(test)]
static VIDEO_MODE_CALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

#[cfg(test)]
static SKIP_VIDEO_MODE_RESIZE: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

#[cfg(test)]
pub(crate) fn video_mode_calls() -> usize {
    VIDEO_MODE_CALLS.load(std::sync::atomic::Ordering::SeqCst)
}

/// Skip the host resize until the guard drops. Tests use this so Alt+F9 can
/// prove it called [`toggle_video_mode`] without changing the terminal.
#[cfg(test)]
pub(crate) fn skip_video_mode_resize() -> VideoModeGuard {
    SKIP_VIDEO_MODE_RESIZE.store(true, std::sync::atomic::Ordering::SeqCst);
    VideoModeGuard
}

#[cfg(test)]
pub(crate) struct VideoModeGuard;

#[cfg(test)]
impl Drop for VideoModeGuard {
    fn drop(&mut self) {
        SKIP_VIDEO_MODE_RESIZE.store(false, std::sync::atomic::Ordering::SeqCst);
    }
}
