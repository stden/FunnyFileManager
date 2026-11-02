//! Purpose: Alt+F9 — ask the host terminal for 25 or 50 rows and report the height it settles on.

use crossterm::execute;
use std::io;

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
