//! Purpose: The per-core CPU meters, kept as counter growth so that runs of
//! neighbouring cores can be drawn as one bar when the header is short of rows.

use std::num::NonZeroUsize;
use std::time::Duration;

use super::super::super::{CpuCounters, SystemSample};
use super::counters::{combine, SystemCounters};
use super::CpuMeter;

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
    pub(in crate::models::tasks::meters) fn per_core(
        previous: Option<&SystemCounters>,
        system: &SystemSample,
    ) -> Self {
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
