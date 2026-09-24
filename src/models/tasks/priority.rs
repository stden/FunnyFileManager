//! Purpose: A process's priority as each platform reports it, and the only
//! changes the task list may ask for.
//!
//! What is *reported* and what may be *set* are separate types: Windows
//! reports a Realtime class, but [`SettableClass`] has no such variant, so no
//! code path can raise a process to it.

mod class;
mod nice;

use super::ControlRefusal;

pub use class::{PriorityClass, SettableClass};
pub use nice::{KernelPriority, NiceOutOfRange, NiceStep, Niceness};

/// A process's priority as its platform reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    Unix {
        kernel: KernelPriority,
        nice: Niceness,
    },
    Windows {
        class: PriorityClass,
    },
    /// The source could not read it.
    Unknown,
}

impl Priority {
    /// The change one `step` from here makes, or why none can be made: a
    /// nice value or a class already at its bound, a Realtime class, or a
    /// priority the source could not read.
    pub fn stepped(self, step: NiceStep) -> Result<PriorityChange, ControlRefusal> {
        match self {
            Self::Unix { nice, .. } => nice
                .step(step)
                .map(PriorityChange::Nice)
                .ok_or(ControlRefusal::AtLimit(nice)),
            Self::Windows { class } => {
                let settable = class.settable().ok_or(ControlRefusal::RealtimeClass)?;
                let stepped = settable.step(step).ok_or(ControlRefusal::ClassAtLimit)?;
                Ok(PriorityChange::Class(stepped))
            }
            Self::Unknown => Err(ControlRefusal::UnknownPriority),
        }
    }
}

/// A priority the task list asks a platform to set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriorityChange {
    Nice(Niceness),
    Class(SettableClass),
}
