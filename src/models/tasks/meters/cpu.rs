//! Purpose: The CPU meters: how each CPU, all of them together, and runs of
//! neighbouring cores spent the time between two samples.
//!
//! Counters are kept as growth until a bar is drawn, so a run of cores can be
//! measured as one CPU — its members' growth added, then divided — rather
//! than as an average of rounded shares.

use std::collections::HashMap;
use std::num::{NonZeroU128, NonZeroUsize};
use std::time::Duration;

use super::super::{CoreSample, CpuCounters, Percent, SystemSample};

/// What one sample's CPU counters leave for the next sample to measure from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::models::tasks) struct SystemCounters {
    total: CpuCounters,
    cores: HashMap<u16, CpuCounters>,
}

impl SystemCounters {
    pub(in crate::models::tasks) fn of(system: &SystemSample) -> Self {
        let cores = system.cores.iter().map(|core| (core.index, core.counters));
        Self {
            total: system.total,
            cores: cores.collect(),
        }
    }

    /// How far every CPU together got since these counters.
    pub(in crate::models::tasks) fn total_growth(&self, system: &SystemSample) -> CpuCounters {
        growth(&system.total, &self.total)
    }

    /// How far `core` got since these counters, or `None` for a core that was
    /// offline then.
    fn core_growth(&self, core: &CoreSample) -> Option<CpuCounters> {
        let before = self.cores.get(&core.index)?;
        Some(growth(&core.counters, before))
    }
}

/// How far each counter got from `before` to `now`; a counter that went
/// backwards counts as zero.
fn growth(now: &CpuCounters, before: &CpuCounters) -> CpuCounters {
    combine(now, before, Duration::saturating_sub)
}

/// `op` applied to each pair of fields.
fn combine(
    left: &CpuCounters,
    right: &CpuCounters,
    op: fn(Duration, Duration) -> Duration,
) -> CpuCounters {
    CpuCounters {
        user: op(left.user, right.user),
        nice: op(left.nice, right.nice),
        system: op(left.system, right.system),
        idle: op(left.idle, right.idle),
        virtualised: op(left.virtualised, right.virtualised),
    }
}

/// All five counters together, in nanoseconds: the time the CPUs they count
/// had, busy or idle.
pub(in crate::models::tasks) fn total_time(counters: &CpuCounters) -> u128 {
    [
        counters.user,
        counters.nice,
        counters.system,
        counters.idle,
        counters.virtualised,
    ]
    .iter()
    .fold(0, |sum, part| sum.saturating_add(part.as_nanos()))
}

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

/// One CPU meter per online core, still as counter growth, so cores can be
/// grouped into one bar when the header has too few rows for one bar each.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpuMeters(Vec<CoreLoad>);

/// One core's growth since the previous sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CoreLoad {
    index: u16,
    /// `None` until the core has been seen in two samples.
    growth: Option<CpuCounters>,
}

impl CpuMeters {
    /// Each online core of `system`, measured against `previous`.
    pub(super) fn per_core(previous: Option<&SystemCounters>, system: &SystemSample) -> Self {
        let cores = system.cores.iter().map(|core| CoreLoad {
            index: core.index,
            growth: previous.and_then(|before| before.core_growth(core)),
        });
        Self(cores.collect())
    }

    /// How many cores were online in the sample.
    #[must_use]
    pub fn count(&self) -> usize {
        self.0.len()
    }

    /// The cores in bars of `per_bar` neighbours each, in core order. A group
    /// counts as one CPU: its members' growth added up, then divided.
    #[must_use]
    pub fn grouped(&self, per_bar: NonZeroUsize) -> Vec<CoreGroup> {
        self.0
            .chunks(per_bar.get())
            .filter_map(|members| {
                let (first, last) = (members.first()?, members.last()?);
                let growth = members
                    .iter()
                    .filter_map(|member| member.growth)
                    .reduce(|sum, member| combine(&sum, &member, Duration::saturating_add));
                Some(CoreGroup {
                    first_core: first.index,
                    last_core: last.index,
                    meter: CpuMeter::from_growth(growth),
                })
            })
            .collect()
    }
}

/// One drawn core bar: a core, or a run of neighbouring cores, and its meter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoreGroup {
    first_core: u16,
    last_core: u16,
    meter: CpuMeter,
}

impl CoreGroup {
    #[must_use]
    pub const fn first_core(self) -> u16 {
        self.first_core
    }

    /// The same as [`CoreGroup::first_core`] for a bar of one core.
    #[must_use]
    pub const fn last_core(self) -> u16 {
        self.last_core
    }

    #[must_use]
    pub const fn meter(self) -> CpuMeter {
        self.meter
    }
}
