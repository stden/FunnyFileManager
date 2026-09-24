//! Purpose: One column of the task table — its title, its fixed width and
//! the key a click on its title sorts by — and the order columns give way in.

use crate::models::{CommandView, SortKey};

use super::super::super::Cells;

/// A column of the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::ui::tasks) enum Column {
    Pid,
    User,
    Priority,
    Nice,
    Virtual,
    Resident,
    Shared,
    State,
    Cpu,
    Memory,
    Time,
    Command,
}

impl Column {
    /// Every column, left to right.
    pub(super) const ALL: [Self; 12] = [
        Self::Pid,
        Self::User,
        Self::Priority,
        Self::Nice,
        Self::Virtual,
        Self::Resident,
        Self::Shared,
        Self::State,
        Self::Cpu,
        Self::Memory,
        Self::Time,
        Self::Command,
    ];

    /// The columns that give way to a short Command column, first to go
    /// first.
    pub(super) const DROP_ORDER: [Self; 9] = [
        Self::Shared,
        Self::Priority,
        Self::Virtual,
        Self::Memory,
        Self::Time,
        Self::Nice,
        Self::User,
        Self::State,
        Self::Resident,
    ];

    pub(super) fn title(self, view: CommandView) -> &'static str {
        match self {
            Self::Pid => "PID",
            Self::User => "User",
            Self::Priority => "PRI",
            Self::Nice => "NI",
            Self::Virtual => "VIRT",
            Self::Resident => "RES",
            Self::Shared => "SHR",
            Self::State => "S",
            Self::Cpu => "CPU%",
            Self::Memory => "MEM%",
            Self::Time => "TIME+",
            Self::Command => match view {
                CommandView::CommandLine => "Command",
                CommandView::Name => "Name",
            },
        }
    }

    /// The width of every column but Command, which takes what is left.
    pub(super) const fn fixed_width(self, pid: Cells) -> Cells {
        match self {
            Self::Pid => pid,
            Self::User => Cells::new(8),
            Self::Priority | Self::Nice => Cells::new(3),
            Self::Virtual | Self::Resident | Self::Shared => Cells::new(6),
            Self::State => Cells::new(1),
            Self::Cpu | Self::Memory => Cells::new(5),
            Self::Time => Cells::new(9),
            Self::Command => Cells::ZERO,
        }
    }

    /// What a click on the column's title sorts by. MEM% sorts as RES and
    /// NI as PRI: the same order.
    pub(in crate::ui::tasks) const fn sort_key(self, view: CommandView) -> SortKey {
        match self {
            Self::Pid => SortKey::Pid,
            Self::User => SortKey::User,
            Self::Priority | Self::Nice => SortKey::Priority,
            Self::Virtual => SortKey::Virtual,
            Self::Resident | Self::Memory => SortKey::Resident,
            Self::Shared => SortKey::Shared,
            Self::State => SortKey::State,
            Self::Cpu => SortKey::Cpu,
            Self::Time => SortKey::CpuTime,
            Self::Command => match view {
                CommandView::CommandLine => SortKey::Command,
                CommandView::Name => SortKey::Name,
            },
        }
    }
}
