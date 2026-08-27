//! Purpose: the real-keyboard side of shell-command cancellation.
//!
//! Split out of `shell.rs` so the coverage policy can exclude it by file name,
//! like `bootstrap.rs`: draining `crossterm`'s event queue requires a live
//! terminal with keys actually pending, which no test environment has. The
//! decision of *what counts as cancel* is [`super::is_cancel_event`], which is
//! pure and fully measured.

use std::time::Duration;

/// True when Esc or Ctrl+C is waiting in the input queue.
///
/// The event loop is blocked on us, so we peek at the queue ourselves.
pub(super) fn cancel_requested() -> bool {
    while crossterm::event::poll(Duration::ZERO).unwrap_or(false) {
        let Ok(event) = crossterm::event::read() else {
            continue;
        };
        if super::is_cancel_event(&event) {
            return true;
        }
    }
    false
}
