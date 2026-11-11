//! Purpose: The Unix side of a priority: the nice value, its bounds and its
//! steps, and the kernel's scheduling priority as `stat` reports it.

use std::fmt;

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
