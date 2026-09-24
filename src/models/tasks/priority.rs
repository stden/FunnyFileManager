//! Purpose: A process's priority as each platform reports it, and the only
//! changes the task list may ask for.
//!
//! What is *reported* and what may be *set* are separate types: Windows
//! reports a Realtime class, but [`SettableClass`] has no such variant, so no
//! code path can raise a process to it.

use std::fmt;

use super::ControlRefusal;

/// A Unix nice value: **-20** (runs first) **..= 19** (runs last).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Niceness(i8);

impl Niceness {
    /// The highest priority, which only root may set.
    pub const MIN: Self = Self(-20);
    /// The lowest priority.
    pub const MAX: Self = Self(19);
    /// The nice value a process runs at unless it, or a parent, changed it.
    pub const NORMAL: Self = Self(0);

    #[must_use]
    pub const fn get(self) -> i8 {
        self.0
    }

    /// The nice value one `step` from here, or `None` at the bound it would
    /// cross.
    #[must_use]
    pub fn step(self, step: NiceStep) -> Option<Self> {
        let next = match step {
            NiceStep::Raise => self.0.checked_sub(1)?,
            NiceStep::Lower => self.0.checked_add(1)?,
        };
        Self::try_from(i32::from(next)).ok()
    }
}

impl TryFrom<i32> for Niceness {
    type Error = NiceOutOfRange;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        i8::try_from(value)
            .ok()
            .filter(|nice| (Self::MIN.0..=Self::MAX.0).contains(nice))
            .map(Self)
            .ok_or(NiceOutOfRange)
    }
}

/// A value that is not a nice value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NiceOutOfRange;

impl fmt::Display for NiceOutOfRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("A nice value is a whole number from -20 to 19.")
    }
}

impl std::error::Error for NiceOutOfRange {}

/// One step of priority, in the direction a user means rather than the sign
/// of the nice value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NiceStep {
    /// Run sooner: nice − 1 (`]`, and Up in the priority form).
    Raise,
    /// Run later: nice + 1 (`[`).
    Lower,
}

/// The kernel's scheduling priority, `stat` field 18: 0..=39 (20 + nice) for
/// normal scheduling, negative for real-time and deadline tasks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct KernelPriority(i32);

impl KernelPriority {
    /// The top real-time priority (`rt_priority` 99) as `stat` reports it.
    const TOP_REALTIME: i32 = -100;

    #[must_use]
    pub const fn new(raw: i32) -> Self {
        Self(raw)
    }

    #[must_use]
    pub const fn get(self) -> i32 {
        self.0
    }

    /// Whether this is the top real-time priority or beyond it (a deadline
    /// task reports −101): the values htop prints as `RT`, because they do not
    /// fit its three-cell column. Lower real-time priorities, −2..=−99, fit
    /// and are shown as their number.
    #[must_use]
    pub const fn is_realtime(self) -> bool {
        self.0 <= Self::TOP_REALTIME
    }
}

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
