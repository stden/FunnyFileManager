//! Purpose: the real-keyboard side of type-ahead — empty crossterm's queue of
//! keys typed while the file manager could not answer, noticing a cancel.
//!
//! The drain takes the queue's `poll` and `read` as parameters, so a test can
//! feed it events without a live terminal; the decision of *what counts as
//! cancel* is [`super::is_cancel_event`].

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
    discard_from(
        || crossterm::event::poll(Duration::ZERO),
        crossterm::event::read,
    )
}

/// The drain [`discard_queued`] runs on crossterm's own queue.
fn discard_from(
    mut poll: impl FnMut() -> std::io::Result<bool>,
    mut read: impl FnMut() -> std::io::Result<crossterm::event::Event>,
) -> bool {
    let mut cancel = false;
    while poll().unwrap_or(false)
        && read().is_ok_and(|event| {
            cancel |= super::is_cancel_event(&event);
            true
        })
    {}
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

#[cfg(test)]
mod tests;
