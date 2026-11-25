//! Purpose: One process as a source reads it — identity, parent, names,
//! owner, state, priority, times and memory — before any share is derived.

use std::time::Duration;

use super::super::{MemoryBytes, Pid, Priority, ProcessKey};

/// One process as read, before any share is derived.
///
/// No `Debug`: `command` is the full command line, which may hold a secret.
#[derive(Clone, PartialEq, Eq)]
pub struct ProcessSample {
    pub key: ProcessKey,
    /// `None` for a process with no parent (PID 1, kernel roots).
    pub parent: Option<Pid>,
    /// The short name the kernel keeps (`comm`), untrusted text.
    pub name: String,
    /// The command line, or `[name]` when there is none; untrusted text.
    pub command: String,
    pub owner: Owner,
    pub state: ProcessState,
    pub priority: Priority,
    pub threads: u32,
    /// User plus system time used since the process started.
    pub cpu_time: Duration,
    pub memory: Option<MemoryUsage>,
    pub running_for: Option<Duration>,
    pub kind: ProcessKind,
}

/// A process's memory, in the sizes htop shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryUsage {
    pub virtual_size: MemoryBytes,
    pub resident: MemoryBytes,
    /// `None` where the platform does not report it.
    pub shared: Option<MemoryBytes>,
}

/// Whose a process is.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Owner {
    Named(String),
    /// A uid with no passwd entry, shown as its number.
    Uid(u32),
    /// The source could not read the owner.
    Unknown,
}

/// What a process is doing, by the kernel's state letter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Running,
    Sleeping,
    /// Uninterruptible sleep, usually disk I/O (`D`).
    DiskSleep,
    Zombie,
    Stopped,
    TracingStop,
    Dead,
    Idle,
    Parked,
    /// A letter this list does not name.
    Other(StateLetter),
}

impl ProcessState {
    /// The state for the kernel's letter. Total: an unknown letter is kept as
    /// [`ProcessState::Other`].
    #[must_use]
    pub const fn from_letter(letter: u8) -> Self {
        match letter {
            b'R' => Self::Running,
            b'S' => Self::Sleeping,
            b'D' => Self::DiskSleep,
            b'Z' => Self::Zombie,
            b'T' => Self::Stopped,
            b't' => Self::TracingStop,
            b'X' => Self::Dead,
            b'I' => Self::Idle,
            b'P' => Self::Parked,
            other => Self::Other(StateLetter::new(other)),
        }
    }

    /// The letter shown in the `S` column: always a visible ASCII character.
    #[must_use]
    pub fn letter(self) -> char {
        let byte = match self {
            Self::Running => b'R',
            Self::Sleeping => b'S',
            Self::DiskSleep => b'D',
            Self::Zombie => b'Z',
            Self::Stopped => b'T',
            Self::TracingStop => b't',
            Self::Dead => b'X',
            Self::Idle => b'I',
            Self::Parked => b'P',
            Self::Other(StateLetter(byte)) => byte,
        };
        char::from(byte)
    }
}

/// An unnamed state letter, safe to draw.
///
/// It is a visible ASCII character, or `?` in place of anything else: the
/// letter comes from a file another process can influence, so a raw control
/// byte never reaches the terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StateLetter(u8);

impl StateLetter {
    const fn new(byte: u8) -> Self {
        if byte.is_ascii_graphic() {
            Self(byte)
        } else {
            Self(b'?')
        }
    }
}

/// Whether a process runs user code or is a kernel thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessKind {
    User,
    KernelThread,
}
