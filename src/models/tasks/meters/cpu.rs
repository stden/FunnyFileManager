//! Purpose: The CPU meters: how each CPU, all of them together, and runs of
//! neighbouring cores spent the time between two samples.
//!
//! Counters are kept as growth until a bar is drawn, so a run of cores can be
//! measured as one CPU — its members' growth added, then divided — rather
//! than as an average of rounded shares.

mod cores;
mod counters;

use std::num::NonZeroU128;

use super::super::{CpuCounters, Percent, SystemSample};

pub use cores::{CoreGroup, CpuMeters};
pub(in crate::models::tasks) use counters::{total_time, SystemCounters};

/// A CPU meter: pending until two samples are apart in time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuMeter {
    /// No earlier counters to measure from, or no time between the two.
    Pending,
    Measured(CpuBar),
}

impl CpuMeter {
    /// All CPUs together, measured against `previous`.
    pub(super) fn total(previous: Option<&SystemCounters>, system: &SystemSample) -> Self {
        Self::from_growth(previous.map(|before| before.total_growth(system)))
    }

    fn from_growth(growth: Option<CpuCounters>) -> Self {
        growth
            .as_ref()
            .and_then(CpuBar::from_growth)
            .map_or(Self::Pending, Self::Measured)
    }
}

/// How a CPU spent the time between two samples, as the bar draws it.
///
/// Each segment ends at the running total of the parts up to and including
/// it, rounded once. The segments therefore add up to exactly the rounded busy
/// share and never to more than [`Percent::FULL`], which rounding every
/// segment on its own could exceed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuBar {
    user: Percent,
    nice: Percent,
    kernel: Percent,
    virtualised: Percent,
}

impl CpuBar {
    /// The bar for `growth`, or `None` when no time passed.
    fn from_growth(growth: &CpuCounters) -> Option<Self> {
        let whole = NonZeroU128::new(total_time(growth))?;
        let mut reached: u128 = 0;
        let mut drawn: u32 = 0;
        let parts = [growth.user, growth.nice, growth.system, growth.virtualised];
        let [user, nice, kernel, virtualised] = parts.map(|part| {
            reached = reached.saturating_add(part.as_nanos());
            let end = Percent::of(reached, whole).hundredths();
            let segment = end.saturating_sub(drawn);
            drawn = end;
            Percent::from_hundredths(segment)
        });
        Some(Self {
            user,
            nice,
            kernel,
            virtualised,
        })
    }

    #[must_use]
    pub const fn user(self) -> Percent {
        self.user
    }

    #[must_use]
    pub const fn nice(self) -> Percent {
        self.nice
    }

    /// System, interrupt and soft-interrupt time.
    #[must_use]
    pub const fn kernel(self) -> Percent {
        self.kernel
    }

    /// Steal and guest time.
    #[must_use]
    pub const fn virtualised(self) -> Percent {
        self.virtualised
    }

    /// The busy share printed on the bar: everything but idle time.
    #[must_use]
    pub fn busy(self) -> Percent {
        let segments = [self.user, self.nice, self.kernel, self.virtualised];
        let hundredths = segments.iter().fold(0_u32, |sum, segment| {
            sum.saturating_add(segment.hundredths())
        });
        Percent::from_hundredths(hundredths)
    }
}
