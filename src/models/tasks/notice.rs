//! Purpose: What the task list tells the user after an action — one fixed
//! English sentence on the info row until the next key — and the line the
//! Error dialog shows for each target a delivery failed for.
//!
//! Sentences name PIDs and short names only: a command line may hold a
//! secret, so it is never quoted here.

use std::fmt;

use super::targets::{ControlRefusal, Target};
use super::{ControlError, NiceStep, Niceness, Owner, Signal};

/// The outcome of an action, shown in place of the info row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Notice {
    Sent {
        signal: Signal,
        count: usize,
    },
    /// The Priority form set `nice` on `count` processes.
    Reniced {
        count: usize,
        nice: Niceness,
    },
    /// `]` or `[` moved `count` processes one step.
    Stepped {
        step: NiceStep,
        count: usize,
    },
    Refused(ControlRefusal),
    /// Gray `+` or `-`: how many marks changed and how many shown rows are
    /// marked now.
    Marked {
        changed: usize,
        total: usize,
    },
}

/// How a notice is coloured.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Dialog colours.
    Information,
    /// Warning colours: a refusal.
    Warning,
}

impl Notice {
    #[must_use]
    pub const fn severity(self) -> Severity {
        match self {
            Self::Sent { .. }
            | Self::Reniced { .. }
            | Self::Stepped { .. }
            | Self::Marked { .. } => Severity::Information,
            Self::Refused(_) => Severity::Warning,
        }
    }
}

impl fmt::Display for Notice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sent { signal, count } => {
                write!(f, "Sent {} to {}.", signal.name(), Processes(*count))
            }
            Self::Reniced { count, nice } => {
                write!(f, "Nice set to {} for {}.", nice.get(), Processes(*count))
            }
            Self::Stepped { step, count } => {
                let way = match step {
                    NiceStep::Raise => "raised",
                    NiceStep::Lower => "lowered",
                };
                write!(f, "Priority {way} one step for {}.", Processes(*count))
            }
            Self::Refused(refusal) => refusal.fmt(f),
            Self::Marked { changed, total } => {
                write!(
                    f,
                    "Marks changed on {}; {total} marked.",
                    Processes(*changed)
                )
            }
        }
    }
}

/// `count` processes, in the singular for one.
struct Processes(usize);

impl fmt::Display for Processes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            1 => f.write_str("1 process"),
            count => write!(f, "{count} processes"),
        }
    }
}

/// What was asked of a target, which decides how its failure reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attempt {
    Signal,
    /// A renice that moves the priority this way: raising it is the one a
    /// user's own process can refuse, since it takes `CAP_SYS_NICE`.
    Renice(NiceStep),
}

impl Attempt {
    /// What "nothing was …" says was not done.
    const fn undone(self) -> &'static str {
        match self {
            Self::Signal => "sent",
            Self::Renice(_) => "changed",
        }
    }

    /// What "Could not … 4242 firefox" says could not be done.
    const fn verb(self) -> &'static str {
        match self {
            Self::Signal => "signal",
            Self::Renice(_) => "renice",
        }
    }

    /// Why a replayed session did nothing.
    const fn replayed(self) -> &'static str {
        match self {
            Self::Signal => "This is a replayed session; processes are not signalled.",
            Self::Renice(_) => "This is a replayed session; priorities are not changed.",
        }
    }
}

/// One line of the Error dialog: a target, what was asked of it, and why it
/// failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    target: Target,
    attempt: Attempt,
    error: ControlError,
}

impl Failure {
    #[must_use]
    pub const fn new(target: Target, attempt: Attempt, error: ControlError) -> Self {
        Self {
            target,
            attempt,
            error,
        }
    }

    #[must_use]
    pub const fn target(&self) -> &Target {
        &self.target
    }

    #[must_use]
    pub const fn attempt(&self) -> Attempt {
        self.attempt
    }

    #[must_use]
    pub const fn error(&self) -> ControlError {
        self.error
    }
}

/// The dialog line: one fixed sentence per error, naming the PID and the
/// short name, never the command line.
impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pid = self.target.key().pid();
        let who = Who(&self.target);
        let undone = self.attempt.undone();
        match self.error {
            ControlError::Gone => write!(f, "{who} has already exited; nothing was {undone}."),
            ControlError::Replaced => write!(
                f,
                "PID {pid} now belongs to another process; nothing was {undone}."
            ),
            ControlError::PermissionDenied => self.fmt_permission(f),
            ControlError::Critical => write!(
                f,
                "{pid} is a critical system process; ending it stops Windows."
            ),
            ControlError::Unverifiable => write!(
                f,
                "FFM cannot confirm which process {pid} is; nothing was {undone}."
            ),
            ControlError::ReadOnly => f.write_str(self.attempt.replayed()),
            ControlError::Refused(refusal) => refusal.fmt(f),
            ControlError::Unsupported => f.write_str("Not supported on this platform."),
            ControlError::Failed(kind) => {
                write!(f, "Could not {} {who}: {kind}.", self.attempt.verb())
            }
        }
    }
}

impl Failure {
    /// The kernel said no: raising a priority takes `CAP_SYS_NICE` even on
    /// the user's own process; anything else was another user's process.
    fn fmt_permission(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.attempt {
            Attempt::Renice(NiceStep::Raise) => {
                f.write_str("Only root can raise a process's priority (CAP_SYS_NICE).")
            }
            Attempt::Signal | Attempt::Renice(NiceStep::Lower) => write!(
                f,
                "{} belongs to another user{}.",
                Who(&self.target),
                OwnerNote(self.target.owner())
            ),
        }
    }
}

/// A target as a sentence names it: `4242 firefox`.
struct Who<'a>(&'a Target);

impl fmt::Display for Who<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.0.key().pid(), self.0.name())
    }
}

/// The owner in brackets after "another user", when it is known.
struct OwnerNote<'a>(&'a Owner);

impl fmt::Display for OwnerNote<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Owner::Named(name) => write!(f, " ({name})"),
            Owner::Uid(uid) => write!(f, " (uid {uid})"),
            Owner::Unknown => Ok(()),
        }
    }
}
