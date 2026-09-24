//! Purpose: The order of the task list — the key, its natural direction and
//! `I`'s reversal — and the comparison every key sorts by, ties always
//! broken by process identity so the order is the same on every refresh.

use std::cmp::Ordering;

use crate::models::SortOrder;

use super::super::{MemoryBytes, MemoryUsage, Priority, PriorityClass, ProcessRow, ProcessSample};

/// What the table can be sorted by. `MEM%` sorts as `Resident`, since the
/// share is resident memory over a total every row shares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SortKey {
    Pid,
    User,
    Priority,
    Virtual,
    Resident,
    Shared,
    State,
    Cpu,
    CpuTime,
    Started,
    Name,
    Command,
}

impl SortKey {
    /// Every key, in the order of the table's columns: the Sort by menu rows.
    pub const ALL: [Self; 12] = [
        Self::Pid,
        Self::User,
        Self::Priority,
        Self::Virtual,
        Self::Resident,
        Self::Shared,
        Self::State,
        Self::Cpu,
        Self::CpuTime,
        Self::Started,
        Self::Name,
        Self::Command,
    ];

    /// The direction a key sorts in before `I` reverses it: the heaviest
    /// processes first for the amounts, ascending for everything else.
    #[must_use]
    pub const fn natural(self) -> SortDirection {
        match self {
            Self::Cpu | Self::Resident | Self::Virtual | Self::Shared | Self::CpuTime => {
                SortDirection::Descending
            }
            Self::Pid
            | Self::User
            | Self::Priority
            | Self::State
            | Self::Started
            | Self::Name
            | Self::Command => SortDirection::Ascending,
        }
    }

    /// `left` against `right` by this key alone, smallest first. An unknown
    /// amount is smaller than every known one.
    fn compare(self, left: &ProcessRow, right: &ProcessRow) -> Ordering {
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

/// Which way a sorted column runs on screen, whatever the key's nature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    /// Smallest first: `▲`.
    Ascending,
    /// Largest first: `▼`.
    Descending,
}

impl SortDirection {
    const fn flipped(self) -> Self {
        match self {
            Self::Ascending => Self::Descending,
            Self::Descending => Self::Ascending,
        }
    }
}

/// The key the table is sorted by, and whether `I` reversed it.
///
/// Ties always fall back to the process identity, PID ascending, so two rows
/// never swap places between refreshes because an unstable sort met equal
/// keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sort {
    key: SortKey,
    /// `Direct` is the key's [natural](SortKey::natural) direction.
    order: SortOrder,
}

impl Default for Sort {
    /// The busiest processes first, as htop opens.
    fn default() -> Self {
        Self::new(SortKey::Cpu, SortOrder::Direct)
    }
}

impl Sort {
    #[must_use]
    pub const fn new(key: SortKey, order: SortOrder) -> Self {
        Self { key, order }
    }

    #[must_use]
    pub const fn key(self) -> SortKey {
        self.key
    }

    #[must_use]
    pub const fn order(self) -> SortOrder {
        self.order
    }

    /// The sort after `key` is chosen — by its letter, its function key, the
    /// menu or a click on its title: the reverse when `key` is already in
    /// force, as Far's sort keys do, and otherwise its natural direction.
    #[must_use]
    pub fn chosen(self, key: SortKey) -> Self {
        if key == self.key {
            self.reversed()
        } else {
            Self::new(key, SortOrder::Direct)
        }
    }

    /// The same key the other way round: `I`.
    #[must_use]
    pub const fn reversed(self) -> Self {
        Self::new(self.key, self.order.toggled())
    }

    /// The direction the rows run in, for the title's arrow.
    #[must_use]
    pub const fn direction(self) -> SortDirection {
        let natural = self.key.natural();
        match self.order {
            SortOrder::Direct => natural,
            SortOrder::Reversed => natural.flipped(),
        }
    }

    pub(super) fn compare(self, left: &ProcessRow, right: &ProcessRow) -> Ordering {
        let by_key = self.key.compare(left, right);
        let directed = match self.direction() {
            SortDirection::Ascending => by_key,
            SortDirection::Descending => by_key.reverse(),
        };
        directed.then_with(|| left.sample().key.cmp(&right.sample().key))
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
    a.chars()
        .flat_map(char::to_lowercase)
        .cmp(b.chars().flat_map(char::to_lowercase))
}
