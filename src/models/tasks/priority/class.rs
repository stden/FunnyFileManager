//! Purpose: The Windows side of a priority: the class the system reports, and
//! the classes the task list may set, which never include Realtime.

use super::NiceStep;

/// A Windows priority class as the system reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriorityClass {
    Idle,
    BelowNormal,
    Normal,
    AboveNormal,
    High,
    Realtime,
}

impl PriorityClass {
    /// The class as one the task list may set, or `None` for Realtime, which
    /// is never changed from here: renicing such a process is refused.
    #[must_use]
    pub const fn settable(self) -> Option<SettableClass> {
        match self {
            Self::Idle => Some(SettableClass::Idle),
            Self::BelowNormal => Some(SettableClass::BelowNormal),
            Self::Normal => Some(SettableClass::Normal),
            Self::AboveNormal => Some(SettableClass::AboveNormal),
            Self::High => Some(SettableClass::High),
            Self::Realtime => None,
        }
    }
}

/// A Windows priority class the task list may set. Realtime is
/// unrepresentable: it can starve the input threads of the whole system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettableClass {
    Idle,
    BelowNormal,
    Normal,
    AboveNormal,
    High,
}

impl SettableClass {
    /// From the class that runs last to the one that runs first.
    const ASCENDING: [Self; 5] = [
        Self::Idle,
        Self::BelowNormal,
        Self::Normal,
        Self::AboveNormal,
        Self::High,
    ];

    /// The class one `step` from here, or `None` past Idle or High.
    #[must_use]
    pub fn step(self, step: NiceStep) -> Option<Self> {
        let here = Self::ASCENDING.iter().position(|class| *class == self)?;
        let next = match step {
            NiceStep::Raise => here.checked_add(1)?,
            NiceStep::Lower => here.checked_sub(1)?,
        };
        Self::ASCENDING.get(next).copied()
    }
}
