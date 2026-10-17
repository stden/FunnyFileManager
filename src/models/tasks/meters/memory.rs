//! Purpose: The memory and swap meters: htop's segments of a total, cut so
//! that together they never exceed it.

use super::super::{MemoryBytes, MemorySample, SwapSample};

/// The memory bar: used, buffers, shared and cache, in that order, together
/// never more than the total.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryMeter {
    total: MemoryBytes,
    used: MemoryBytes,
    buffers: MemoryBytes,
    shared: MemoryBytes,
    cache: MemoryBytes,
}

impl MemoryMeter {
    pub(super) fn from_sample(memory: &MemorySample) -> Self {
        let cache = memory
            .cached
            .saturating_add(memory.reclaimable)
            .saturating_sub(memory.shared);
        let segments = [used_memory(memory), memory.buffers, memory.shared, cache];
        let [used, buffers, shared, cache] = fit_in(memory.total, segments);
        Self {
            total: memory.total,
            used,
            buffers,
            shared,
            cache,
        }
    }

    #[must_use]
    pub const fn total(self) -> MemoryBytes {
        self.total
    }

    #[must_use]
    pub const fn used(self) -> MemoryBytes {
        self.used
    }

    #[must_use]
    pub const fn buffers(self) -> MemoryBytes {
        self.buffers
    }

    #[must_use]
    pub const fn shared(self) -> MemoryBytes {
        self.shared
    }

    #[must_use]
    pub const fn cache(self) -> MemoryBytes {
        self.cache
    }
}

/// Memory in use, as htop 3 counts it: what is neither free, cache, reclaimable
/// slab nor buffers, or everything not free when those add up to more than the
/// total. procps `free` would say `total − available` instead; that change is
/// this one function.
fn used_memory(memory: &MemorySample) -> MemoryBytes {
    let unused = [memory.cached, memory.reclaimable, memory.buffers]
        .into_iter()
        .fold(memory.free, MemoryBytes::saturating_add);
    if unused <= memory.total {
        memory.total.saturating_sub(unused)
    } else {
        memory.total.saturating_sub(memory.free)
    }
}

/// The swap bar: used and cache, together never more than the total.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwapMeter {
    total: MemoryBytes,
    used: MemoryBytes,
    cache: MemoryBytes,
}

impl SwapMeter {
    pub(super) fn from_sample(swap: &SwapSample) -> Self {
        let used = swap
            .total
            .saturating_sub(swap.free)
            .saturating_sub(swap.cached);
        let [used, cache] = fit_in(swap.total, [used, swap.cached]);
        Self {
            total: swap.total,
            used,
            cache,
        }
    }

    #[must_use]
    pub const fn total(self) -> MemoryBytes {
        self.total
    }

    #[must_use]
    pub const fn used(self) -> MemoryBytes {
        self.used
    }

    #[must_use]
    pub const fn cache(self) -> MemoryBytes {
        self.cache
    }
}

/// `segments`, each cut to what is left of `total` after the ones before it.
fn fit_in<const N: usize>(total: MemoryBytes, segments: [MemoryBytes; N]) -> [MemoryBytes; N] {
    let mut room = total;
    segments.map(|segment| {
        let kept = segment.min(room);
        room = room.saturating_sub(kept);
        kept
    })
}
