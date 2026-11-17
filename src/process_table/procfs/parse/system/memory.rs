//! Purpose: The parser of `/proc/meminfo` into the memory and swap meters:
//! `Key:  <value> kB` lines in any order, each key read the first time it
//! appears, and the available amount estimated where the kernel is too old
//! to report it.

use crate::models::{MemoryBytes, MemorySample, SwapSample};

use super::super::{complete_lines, unsigned};

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
