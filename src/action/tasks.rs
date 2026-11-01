//! Purpose: The task list's keys (`Ctrl+W`): one pure mapping from a key to
//! a [`TaskCommand`], and the rows of the list's help page.
//!
//! Inside the list a key keeps the meaning Far or ffm gives it wherever that
//! meaning fits a process list; htop's meaning fills the keys Far leaves
//! empty; and every htop key that loses lands on a harmless dialog, so no
//! reflex from either program ends a process without a second, explicit
//! step. Resolving a key has no effect: `popup::tasks` applies the command.

use crate::models::{ListMotion, MaskPurpose, NiceStep, SortKey};
use crate::scroll::WIDE_STEP;

/// What one key asks of the task list while nothing is open on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskCommand {
    /// Up, Down, `PgUp`, `PgDn`, Home, End.
    Move(ListMotion),
    /// Left, Right, Ctrl+Left, Ctrl+Right: the Command column sideways.
    Pan(Pan),
    /// Space, Insert: mark or unmark the process, then move down.
    ToggleMark,
    /// `c`: mark the process and everything descending from it.
    MarkFamily,
    /// `U`.
    UnmarkAll,
    /// `*`: invert the marks of the shown rows.
    InvertMarks,
    /// Gray `+` and `-`: the Select or Deselect dialog, a Far mask over
    /// process names.
    MarkByMask(MaskPurpose),
    /// Enter, F3: the Process or Network information dialog.
    Info,
    /// F1, `h`, `?`.
    Help,
    /// F2: the command line or the name in the last column (Active/All on Network).
    ToggleCommandView,
    /// F4, `\`: the filter bar.
    Filter,
    /// `/` opens the search bar empty; Alt+character opens it with that
    /// character typed.
    Search(Option<char>),
    /// A digit starts a PID jump with that digit typed.
    PidJump(char),
    /// F5, `t`, Ctrl+T: the list or the parent tree.
    ToggleTree,
    /// F6, Ctrl+F12, `<`, `>`: the Sort by menu.
    SortMenu,
    /// A direct sort key; the key already in force reverses the order.
    SortBy(SortKey),
    /// `I`: the same key the other way round.
    InvertOrder,
    /// `u`: the Show processes of menu.
    UserMenu,
    /// `K`: kernel threads shown or hidden.
    ToggleKernelThreads,
    /// F7: the Priority form.
    Priority,
    /// `]` raises the priority one step at once; `[` lowers it, at once for
    /// root and after a confirmation for anyone else, who cannot undo it.
    Nice(NiceStep),
    /// F8, Del: the Terminate confirmation. Never a signal by itself.
    Terminate,
    /// F9, `k`: the Send signal menu, then the same confirmation.
    SignalMenu,
    /// F11, `Z`: hold or resume the refresh.
    ToggleHold,
    /// Tab, F12: switch between the Processes and Network views.
    ToggleTab,
    /// Ctrl+R, Ctrl+L: one sample now.
    RefreshNow,
    /// Esc, F10, `q`, Ctrl+C, Ctrl+W: close the list. Never quits ffm.
    Close,
    /// Every other key: consumed, so nothing falls through to the panels.
    Ignore,
}

/// A sideways scroll of the last column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pan {
    Left(PanStep),
    Right(PanStep),
}

/// How far one pan goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanStep {
    /// Left, Right.
    Cell,
    /// Ctrl+Left, Ctrl+Right: the F3 viewer's wide step.
    Wide,
}

impl PanStep {
    /// The cells this step pans by.
    #[must_use]
    pub const fn cells(self) -> usize {
        match self {
            Self::Cell => 1,
            Self::Wide => WIDE_STEP,
        }
    }
}

mod help;
mod keys;
pub mod search;

pub use help::HELP;
pub use keys::{normalize, resolve};

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_help;
