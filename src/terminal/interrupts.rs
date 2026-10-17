//! Purpose: Keep FFM alive when Ctrl+C or Ctrl+\ is pressed while a program
//! it handed the terminal to is running.
//!
//! Raw mode clears the terminal's ISIG flag, so while the panels are up Ctrl+C
//! is just a key. A [`super::TerminalSuspension`] turns ISIG back on so the
//! program it runs can be interrupted, and that program runs in FFM's own
//! foreground process group: the terminal sends the signal to FFM as well, and
//! the default action used to kill FFM with the terminal still in the
//! program's hands.
//!
//! The signals are *caught*, not ignored: `SIG_IGN` would be inherited by the
//! program, while `exec` resets a caught signal to its default, so the program
//! still stops (or handles the key itself, as sudo and apt-get do). FFM only
//! notes that the key was pressed. Outside a hand-off the default action is
//! emulated, so an interrupt sent to FFM then ends it exactly as before.

use std::io;

/// While alive, Ctrl+C and Ctrl+\ stop the program that owns the terminal and
/// leave FFM running.
///
/// One at a time: the event loop is blocked for as long as a program owns the
/// terminal, so hand-offs never overlap.
pub struct InterruptShield {
    #[cfg(unix)]
    flags: &'static unix::Flags,
}

#[cfg(unix)]
impl InterruptShield {
    pub(crate) fn raise() -> io::Result<Self> {
        let flags = unix::flags()?;
        flags.raise();
        Ok(Self { flags })
    }

    /// Whether Ctrl+C or Ctrl+\ reached FFM since the shield went up.
    pub(crate) fn interrupted(&self) -> bool {
        self.flags.interrupted()
    }
}

#[cfg(unix)]
impl Drop for InterruptShield {
    fn drop(&mut self) {
        self.flags.lower();
    }
}

/// Windows delivers Ctrl+C through the console, not through a process group
/// FFM shares with the program; nothing is caught there.
#[cfg(not(unix))]
impl InterruptShield {
    #[expect(
        clippy::unnecessary_wraps,
        reason = "the same signature as the Unix shield, whose handlers can fail to install"
    )]
    pub(crate) fn raise() -> io::Result<Self> {
        Ok(Self {})
    }

    #[expect(
        clippy::unused_self,
        reason = "the same signature as the Unix shield, which reads its flags"
    )]
    pub(crate) const fn interrupted(&self) -> bool {
        false
    }
}

#[cfg(unix)]
mod unix {
    use std::io;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, OnceLock};

    use signal_hook::consts::{SIGINT, SIGQUIT};
    use signal_hook::flag;

    /// The signals a terminal sends for Ctrl+C and Ctrl+\.
    const INTERRUPTS: [i32; 2] = [SIGINT, SIGQUIT];

    /// What the process-wide handlers read and write.
    pub(super) struct Flags {
        /// Set by the handler whenever an interrupt arrives.
        received: Arc<AtomicBool>,
        /// True outside a hand-off: the handler then runs the default action.
        terminate: Arc<AtomicBool>,
    }

    /// The handlers, installed on the first hand-off and kept for the life of
    /// the process: a signal handler cannot be handed back cleanly, so the
    /// `terminate` flag switches it between "note it" and "die as before".
    pub(super) fn flags() -> io::Result<&'static Flags> {
        static FLAGS: OnceLock<io::Result<Flags>> = OnceLock::new();
        FLAGS
            .get_or_init(Flags::install)
            .as_ref()
            .map_err(|error| io::Error::new(error.kind(), error.to_string()))
    }

    impl Flags {
        fn install() -> io::Result<Self> {
            let flags = Self {
                received: Arc::new(AtomicBool::new(false)),
                terminate: Arc::new(AtomicBool::new(true)),
            };
            for signal in INTERRUPTS {
                flag::register(signal, Arc::clone(&flags.received))?;
                flag::register_conditional_default(signal, Arc::clone(&flags.terminate))?;
            }
            Ok(flags)
        }

        pub(super) fn raise(&self) {
            self.received.store(false, Ordering::SeqCst);
            self.terminate.store(false, Ordering::SeqCst);
        }

        pub(super) fn lower(&self) {
            self.terminate.store(true, Ordering::SeqCst);
        }

        pub(super) fn interrupted(&self) -> bool {
            self.received.load(Ordering::SeqCst)
        }
    }
}
