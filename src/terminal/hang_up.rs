//! Purpose: Catch SIGHUP and SIGTERM so the session ends the way F10 ends it,
//! with what FFM started stopped first, instead of FFM dying on the spot.
//!
//! A command from the command line runs in a process group of its own, so a
//! timeout or Esc can stop all of it. That group is not the terminal's
//! foreground group, so nobody tells it the terminal went away: when SIGHUP
//! killed FFM outright, the command ran on, re-parented to init. Caught, the
//! signal is only noted here; the wait for the terminal then reports the
//! terminal lost ([`crate::recording::TerminalLost`]), a running command is
//! stopped as a shell stops its jobs, and the event loop quits.
//!
//! SIGTERM is the same request made politely: `su` and `runuser` hand it on
//! when their own terminal goes, and `kill` sends it. A second signal of
//! either kind ends FFM at once, as the first one used to.
//!
//! Unix only: Windows ends a console's processes with the console, and has no
//! such signal to catch.

use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

use signal_hook::consts::{SIGHUP, SIGTERM};
use signal_hook::flag;

/// A hang-up, and the polite request to stop.
const ENDINGS: [i32; 2] = [SIGHUP, SIGTERM];

/// Start catching the signals that end a session. Once per process; a
/// second call adds nothing.
///
/// # Errors
///
/// The kernel refused a handler.
pub fn watch_for_hang_up() -> io::Result<()> {
    static WATCHING: OnceLock<io::Result<()>> = OnceLock::new();
    WATCHING
        .get_or_init(install)
        .as_ref()
        .map_err(|error| io::Error::new(error.kind(), error.to_string()))
        .copied()
}

/// Whether SIGHUP or SIGTERM has arrived since [`watch_for_hang_up`].
pub fn hung_up() -> bool {
    received().load(Ordering::SeqCst)
}

/// Set by the handlers; never cleared.
fn received() -> &'static Arc<AtomicBool> {
    static RECEIVED: OnceLock<Arc<AtomicBool>> = OnceLock::new();
    RECEIVED.get_or_init(|| Arc::new(AtomicBool::new(false)))
}

/// The default action comes first, armed by the flag the second handler
/// sets: the first signal is only noted, a second one ends the process.
fn install() -> io::Result<()> {
    for signal in ENDINGS {
        flag::register_conditional_default(signal, Arc::clone(received()))?;
        flag::register(signal, Arc::clone(received()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
