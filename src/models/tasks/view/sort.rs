//! Purpose: The order of the task list — the key, its natural direction and
//! `I`'s reversal — and the comparison every key sorts by, ties always
//! broken by process identity so the order is the same on every refresh.

mod compare;

use std::cmp::Ordering;

use crate::models::SortOrder;

use super::super::ProcessRow;

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
