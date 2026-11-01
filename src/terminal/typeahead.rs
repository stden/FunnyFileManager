//! Purpose: the real-keyboard side of type-ahead — empty crossterm's queue of
//! keys typed while the file manager could not answer, noticing a cancel.
//!
//! Kept in its own file so the coverage policy can exclude it by file name,
//! like `bootstrap.rs`: draining crossterm's event queue requires a live
//! terminal with keys actually pending, which no test environment has. The
//! decision of *what counts as cancel* is [`super::is_cancel_event`], which is
//! pure and fully measured.

use std::time::Duration;

/// Read and drop every event crossterm already holds; whether one of them
/// was the cancel gesture.
///
/// Keys typed at a screen that did not answer were aimed at a screen the
/// user never saw, so none of them may run once it answers again — the
/// guest sweep lost a file to an F8 Enter typed during a long copy. Nothing
/// is recorded: a replay must not run keys the session itself threw away. A
/// read that fails ends the drain; the event loop's next read reports it.
pub fn discard_queued() -> bool {
    if let Some(cancelled) = super::headless::cancelled() {
        return cancelled;
    }
    let mut cancel = false;
    while crossterm::event::poll(Duration::ZERO).unwrap_or(false) {
        let Ok(event) = crossterm::event::read() else {
            break;
        };
        cancel |= super::is_cancel_event(&event);
    }
    cancel
}

/// True when Esc or Ctrl+C is waiting in the input queue; every other key
/// waiting there is dropped with it.
///
/// A running `:` command blocks the event loop, so the command peeks at the
/// queue itself. A terminal that has gone away answers yes: crossterm's
/// reader would otherwise spin on a descriptor that stays readable and gives
/// back nothing, and nothing else would ever notice. Cancelling kills the
/// child and unwinds to the loop, which ends the session on its own next
/// read. See `recording::tty`.
pub fn cancel_requested() -> bool {
    if let Some(cancelled) = super::headless::cancelled() {
        return cancelled;
    }
    crate::recording::wait_for_terminal(Duration::ZERO).is_err() || discard_queued()
}
