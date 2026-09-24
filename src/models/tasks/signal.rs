//! Purpose: The signals the task list offers, by name and purpose, and what
//! the kill confirmation offers to send.
//!
//! Signal *numbers* differ between Unix systems (`SIGUSR1` is 10 on Linux and
//! 30 on macOS), so none appears here: the platform layer maps each variant to
//! its system's number in one exhaustive `match`.

/// A POSIX signal that every Unix target defines. The Linux-only `SIGSTKFLT`
/// and `SIGPWR` are left out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Signal {
    Terminate,
    Kill,
    Hangup,
    Interrupt,
    Quit,
    Abort,
    Stop,
    Continue,
    TerminalStop,
    User1,
    User2,
    Alarm,
    WindowChange,
}

impl Signal {
    /// The signals the Send signal menu lists, SIGTERM first.
    #[cfg(unix)]
    pub const OFFERED: &[Self] = &[
        Self::Terminate,
        Self::Kill,
        Self::Hangup,
        Self::Interrupt,
        Self::Quit,
        Self::Abort,
        Self::Stop,
        Self::Continue,
        Self::TerminalStop,
        Self::User1,
        Self::User2,
        Self::Alarm,
        Self::WindowChange,
    ];

    /// None: Windows has no signals, so F9 offers what F8 does.
    #[cfg(not(unix))]
    pub const OFFERED: &[Self] = &[];

    /// The conventional name, `SIGTERM`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Terminate => "SIGTERM",
            Self::Kill => "SIGKILL",
            Self::Hangup => "SIGHUP",
            Self::Interrupt => "SIGINT",
            Self::Quit => "SIGQUIT",
            Self::Abort => "SIGABRT",
            Self::Stop => "SIGSTOP",
            Self::Continue => "SIGCONT",
            Self::TerminalStop => "SIGTSTP",
            Self::User1 => "SIGUSR1",
            Self::User2 => "SIGUSR2",
            Self::Alarm => "SIGALRM",
            Self::WindowChange => "SIGWINCH",
        }
    }

    /// What sending it usually does, for the menu row beside the name.
    #[must_use]
    pub const fn purpose(self) -> &'static str {
        match self {
            Self::Terminate => "ask to terminate",
            Self::Kill => "end at once; cannot be caught",
            Self::Hangup => "hang up; daemons often reload",
            Self::Interrupt => "interrupt, as Ctrl+C does",
            Self::Quit => "quit and dump core",
            Self::Abort => "abort and dump core",
            Self::Stop => "pause; cannot be caught",
            Self::Continue => "resume a paused process",
            Self::TerminalStop => "pause, as Ctrl+Z does",
            Self::User1 => "user-defined signal 1",
            Self::User2 => "user-defined signal 2",
            Self::Alarm => "timer alarm",
            Self::WindowChange => "terminal size changed",
        }
    }
}

/// What a kill confirmation offers to send.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KillOffer {
    /// F8 or Del: Terminate or Kill, Terminate focused.
    TerminateOrKill,
    /// The signal chosen in the F9 menu.
    Send(Signal),
}
