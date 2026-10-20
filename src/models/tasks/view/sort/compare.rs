//! Purpose: The comparison each sort key orders two rows by, smallest first,
//! with an unknown amount below every known one.

use std::cmp::Ordering;

use super::super::super::{
    MemoryBytes, MemoryUsage, Priority, PriorityClass, ProcessRow, ProcessSample,
};
use super::SortKey;

impl SortKey {
    /// `left` against `right` by this key alone, smallest first. An unknown
    /// amount is smaller than every known one.
    pub(super) fn compare(self, left: &ProcessRow, right: &ProcessRow) -> Ordering {
        let (a, b) = (left.sample(), right.sample());
        match self {
            Self::Pid => a.key.pid().cmp(&b.key.pid()),
            Self::User => a.owner.cmp(&b.owner),
            Self::Priority => priority_rank(a.priority).cmp(&priority_rank(b.priority)),
            Self::Virtual => by_memory(a, b, |usage| Some(usage.virtual_size)),
            Self::Resident => by_memory(a, b, |usage| Some(usage.resident)),
            Self::Shared => by_memory(a, b, |usage| usage.shared),
            Self::State => a.state.letter().cmp(&b.state.letter()),
            Self::Cpu => left.cpu().cmp(&right.cpu()),
            Self::CpuTime => a.cpu_time.cmp(&b.cpu_time),
            Self::Started => a.key.start().cmp(&b.key.start()),
            Self::Name => caseless(&a.name, &b.name),
            Self::Command => caseless(&a.command, &b.command),
        }
    }
}

/// `a` against `b` by one memory figure; an unknown figure is the smallest.
fn by_memory(
    a: &ProcessSample,
    b: &ProcessSample,
    figure: impl Fn(MemoryUsage) -> Option<MemoryBytes>,
) -> Ordering {
    a.memory.and_then(&figure).cmp(&b.memory.and_then(&figure))
}

/// Where a priority falls, runs-first smallest: the kernel's priority, then
/// nice; a Windows class; unknown last. One source never mixes the first two.
fn priority_rank(priority: Priority) -> (u8, i32, i8) {
    match priority {
        Priority::Unix { kernel, nice } => (0, kernel.get(), nice.get()),
        Priority::Windows { class } => (1, class_rank(class), 0),
        Priority::Unknown => (2, 0, 0),
    }
}

const fn class_rank(class: PriorityClass) -> i32 {
    match class {
        PriorityClass::Realtime => 0,
        PriorityClass::High => 1,
        PriorityClass::AboveNormal => 2,
        PriorityClass::Normal => 3,
        PriorityClass::BelowNormal => 4,
        PriorityClass::Idle => 5,
    }
}

/// `a` against `b`, ignoring case, without allocating.
fn caseless(a: &str, b: &str) -> Ordering {
    if a.is_ascii() && b.is_ascii() {
        return a
            .bytes()
            .map(|byte| byte.to_ascii_lowercase())
            .cmp(b.bytes().map(|byte| byte.to_ascii_lowercase()));
    }
    a.chars()
        .flat_map(char::to_lowercase)
        .cmp(b.chars().flat_map(char::to_lowercase))
}
