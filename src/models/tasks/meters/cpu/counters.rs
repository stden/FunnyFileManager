//! Purpose: What one sample's CPU counters leave for the next sample to
//! measure from, and the counter maths the CPU meters share: growth between
//! two samples, field-wise sums, and the total time.

use std::collections::HashMap;
use std::time::Duration;

use super::super::super::{CoreSample, CpuCounters, SystemSample};

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
    pub(super) fn core_growth(&self, core: &CoreSample) -> Option<CpuCounters> {
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
pub(super) fn combine(
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
