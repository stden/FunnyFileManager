//! Purpose: What the task list tells the user after an action — one fixed
//! English sentence on the info row until the next key — and the line the
//! Error dialog shows for each target a delivery failed for.
//!
//! Sentences name PIDs and short names only: a command line may hold a
//! secret, so it is never quoted here.

mod failure;

use std::fmt;

use super::targets::ControlRefusal;
use super::{NiceStep, Niceness, Signal, TaskList};

pub use failure::{Attempt, Failure};

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

impl TaskList {
    /// The notice shown in place of the info row, until the next key.
    #[must_use]
    pub const fn notice(&self) -> Option<Notice> {
        self.notice
    }

    pub const fn set_notice(&mut self, notice: Notice) {
        self.notice = Some(notice);
    }

    pub const fn clear_notice(&mut self) {
        self.notice = None;
    }
}
