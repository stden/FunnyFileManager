//! Purpose: Pure parsers for the system-wide files `/proc` keeps —
//! `/proc/stat`, `/proc/meminfo`, `/proc/loadavg` and `/proc/uptime` — into
//! the counters one sample carries.
//!
//! The same rules as the per-process parsers: `None` rather than a guess,
//! integer arithmetic that saturates or is checked, and no indexing.

use std::time::Duration;

use crate::models::{
    CoreSample, CpuCounters, Load, LoadAverage, MemoryBytes, MemorySample, SwapSample,
};

use super::{complete_lines, unsigned, ClockTicks};

/// What `/proc/stat` says: every CPU's times, each online core's, and how
/// many tasks were runnable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::process_table::procfs) struct CpuTimes {
    pub(in crate::process_table::procfs) total: CpuCounters,
    /// In core order, one per core number.
    pub(in crate::process_table::procfs) cores: Vec<CoreSample>,
    pub(in crate::process_table::procfs) running: Option<u32>,
}

/// A `cpu` line's fields, in ticks, in the kernel's order.
const USER: usize = 0;
const NICE: usize = 1;
const SYSTEM: usize = 2;
const IDLE: usize = 3;
const IOWAIT: usize = 4;
const IRQ: usize = 5;
const SOFTIRQ: usize = 6;
const STEAL: usize = 7;
const GUEST: usize = 8;
const GUEST_NICE: usize = 9;
/// Fewer fields than this is not a CPU line: user, nice, system and idle.
const REQUIRED_CPU_FIELDS: usize = 4;

/// `/proc/stat`, or `None` when the aggregate `cpu` line is missing or any
/// CPU line is malformed: then the sample fails, since CPU shares are what
/// the table is for.
pub(in crate::process_table::procfs) fn cpu_times(
    text: &str,
    ticks: ClockTicks,
) -> Option<CpuTimes> {
    let mut total = None;
    let mut cores = Vec::new();
    let mut running = None;
    for line in complete_lines(text).lines() {
        let mut fields = line.split_ascii_whitespace();
        let Some(label) = fields.next() else {
            continue;
        };
        if label == "procs_running" {
            running = running.or_else(|| unsigned(fields.next()?));
        } else if label == "cpu" {
            let counters = counters(fields, ticks)?;
            total = total.or(Some(counters));
        } else if let Some(number) = core_number(label) {
            let index = unsigned(number)?;
            cores.push(CoreSample {
                index,
                counters: counters(fields, ticks)?,
            });
        }
    }
    cores.sort_by_key(|core| core.index);
    cores.dedup_by_key(|core| core.index);
    Some(CpuTimes {
        total: total?,
        cores,
        running,
    })
}

/// The number of a `cpuN` label: `Some("12")` for `cpu12`, `None` for any
/// label that does not name a core.
fn core_number(label: &str) -> Option<&str> {
    let number = label.strip_prefix("cpu")?;
    let digits = !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit());
    digits.then_some(number)
}

/// One CPU line's ticks folded into the parts a meter draws. Guest time is
/// already counted inside user and nice time, so it is taken out of them and
/// counted once, as virtualised time; missing trailing fields are zero.
fn counters<'a>(fields: impl Iterator<Item = &'a str>, ticks: ClockTicks) -> Option<CpuCounters> {
    let values: Vec<u64> = fields.map(unsigned).collect::<Option<_>>()?;
    if values.len() < REQUIRED_CPU_FIELDS {
        return None;
    }
    let field = |at: usize| values.get(at).copied().unwrap_or(0);
    let sum = |parts: &[usize]| {
        parts
            .iter()
            .fold(0_u64, |sum, &at| sum.saturating_add(field(at)))
    };
    Some(CpuCounters {
        user: ticks.duration(field(USER).saturating_sub(field(GUEST))),
        nice: ticks.duration(field(NICE).saturating_sub(field(GUEST_NICE))),
        system: ticks.duration(sum(&[SYSTEM, IRQ, SOFTIRQ])),
        idle: ticks.duration(sum(&[IDLE, IOWAIT])),
        virtualised: ticks.duration(sum(&[STEAL, GUEST, GUEST_NICE])),
    })
}

/// What `/proc/meminfo` says about memory and swap; each is `None` without
/// its total.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::process_table::procfs) struct MemoryInfo {
    pub(in crate::process_table::procfs) memory: Option<MemorySample>,
    pub(in crate::process_table::procfs) swap: Option<SwapSample>,
}

/// The `/proc/meminfo` keys read, each the first time it appears.
#[derive(Default)]
struct Keys {
    total: Option<MemoryBytes>,
    free: Option<MemoryBytes>,
    available: Option<MemoryBytes>,
    buffers: Option<MemoryBytes>,
    cached: Option<MemoryBytes>,
    reclaimable: Option<MemoryBytes>,
    shared: Option<MemoryBytes>,
    swap_total: Option<MemoryBytes>,
    swap_free: Option<MemoryBytes>,
    swap_cached: Option<MemoryBytes>,
}

impl Keys {
    fn slot(&mut self, key: &str) -> Option<&mut Option<MemoryBytes>> {
        Some(match key {
            "MemTotal" => &mut self.total,
            "MemFree" => &mut self.free,
            "MemAvailable" => &mut self.available,
            "Buffers" => &mut self.buffers,
            "Cached" => &mut self.cached,
            "SReclaimable" => &mut self.reclaimable,
            "Shmem" => &mut self.shared,
            "SwapTotal" => &mut self.swap_total,
            "SwapFree" => &mut self.swap_free,
            "SwapCached" => &mut self.swap_cached,
            _ => return None,
        })
    }
}

/// `/proc/meminfo`: `Key:  <value> kB` lines in any order, `kB` meaning KiB.
/// A missing key counts as zero; a missing `MemAvailable` (kernels before
/// 3.14) is estimated as free plus buffers plus cache.
pub(in crate::process_table::procfs) fn memory_info(text: &str) -> MemoryInfo {
    let mut keys = Keys::default();
    for line in complete_lines(text).lines() {
        let Some((key, rest)) = line.split_once(':') else {
            continue;
        };
        let amount = rest.split_ascii_whitespace().next().and_then(unsigned);
        if let (Some(slot), Some(kib)) = (keys.slot(key), amount) {
            *slot = slot.or_else(|| MemoryBytes::from_kib(kib));
        }
    }
    MemoryInfo {
        memory: memory_sample(&keys),
        swap: swap_sample(&keys),
    }
}

fn memory_sample(keys: &Keys) -> Option<MemorySample> {
    let zero = MemoryBytes::from_bytes(0);
    let [free, buffers, cached] =
        [keys.free, keys.buffers, keys.cached].map(|key| key.unwrap_or(zero));
    let estimate = || free.saturating_add(buffers).saturating_add(cached);
    Some(MemorySample {
        total: keys.total?,
        free,
        available: keys.available.unwrap_or_else(estimate),
        buffers,
        cached,
        reclaimable: keys.reclaimable.unwrap_or(zero),
        shared: keys.shared.unwrap_or(zero),
    })
}

fn swap_sample(keys: &Keys) -> Option<SwapSample> {
    let zero = MemoryBytes::from_bytes(0);
    Some(SwapSample {
        total: keys.swap_total?,
        free: keys.swap_free.unwrap_or(zero),
        cached: keys.swap_cached.unwrap_or(zero),
    })
}

/// `/proc/loadavg`: `0.52 0.58 0.59 3/1204 4242`, the three loads in
/// hundredths, without floating point.
pub(in crate::process_table::procfs) fn load_average(text: &str) -> Option<LoadAverage> {
    let mut fields = complete_lines(text).split_ascii_whitespace();
    let mut next = || {
        u32::try_from(hundredths(fields.next()?)?)
            .ok()
            .map(Load::from_hundredths)
    };
    Some(LoadAverage::new(next()?, next()?, next()?))
}

/// `/proc/uptime`: seconds since boot, to the hundredth, from its first field.
pub(in crate::process_table::procfs) fn uptime(text: &str) -> Option<Duration> {
    let hundredths = hundredths(complete_lines(text).split_ascii_whitespace().next()?)?;
    let seconds = Duration::from_secs(hundredths / 100);
    Some(seconds.saturating_add(Duration::from_millis((hundredths % 100) * 10)))
}

/// A decimal like `0.52` or `21126.06` in hundredths: the whole part times a
/// hundred, plus the first two digits of the fraction; `None` for anything
/// that is not digits, or too large.
fn hundredths(text: &str) -> Option<u64> {
    let (whole, fraction) = text.split_once('.').unwrap_or((text, ""));
    let whole: u64 = unsigned(whole)?;
    if !fraction.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let mut digits = fraction
        .chars()
        .map(|digit| u64::from(digit.to_digit(10).unwrap_or(0)));
    let tenths = digits.next().unwrap_or(0);
    let cents = digits.next().unwrap_or(0);
    whole.checked_mul(100)?.checked_add(tenths * 10 + cents)
}
