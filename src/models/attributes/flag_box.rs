//! Purpose: One three-state checkbox of the attributes dialog: what it showed
//! when the dialog opened, what it shows now, and what that asks for.

use super::{EntrySnapshot, Flag};

/// The three states Far draws a checkbox in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Check {
    /// `[x]`: every selected entry has the bit.
    Set,
    /// `[ ]`: none does.
    Clear,
    /// `[?]`: they disagree — leave each as it is.
    Mixed,
}

impl Check {
    /// The character inside the brackets.
    #[must_use]
    pub const fn mark(self) -> char {
        match self {
            Self::Set => 'x',
            Self::Clear => ' ',
            Self::Mixed => '?',
        }
    }

    fn of(with_bit: usize, total: usize) -> Self {
        if with_bit == 0 {
            Self::Clear
        } else if with_bit == total {
            Self::Set
        } else {
            Self::Mixed
        }
    }
}

/// One checkbox: what it showed when the dialog opened, and what it shows now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlagBox {
    pub flag: Flag,
    pub initial: Check,
    pub current: Check,
    /// Whether the cycle passes through `[?]`. Far makes every box three-state
    /// as soon as a folder is selected, because "leave it" is then a real
    /// answer for the files inside.
    three_state: bool,
}

impl FlagBox {
    /// Space: the next state round. A two-state box flips; a three-state one
    /// goes set → clear → leave → set, so `[?]` is reachable again after a
    /// slip.
    pub fn toggle(&mut self) {
        self.current = match self.current {
            Check::Set => Check::Clear,
            Check::Clear if self.three_state => Check::Mixed,
            Check::Clear | Check::Mixed => Check::Set,
        };
    }

    /// What this box asks for: set the bit, clear it, or nothing.
    ///
    /// `None` both for `[?]` and for a box left as it opened — Far changes
    /// only what the user moved.
    #[must_use]
    pub fn request(&self) -> Option<bool> {
        if self.current == self.initial {
            return None;
        }
        match self.current {
            Check::Set => Some(true),
            Check::Clear => Some(false),
            Check::Mixed => None,
        }
    }
}

/// One box per platform flag, each showing what the selection agrees on.
pub(super) fn boxes(entries: &[EntrySnapshot], has_directories: bool) -> Vec<FlagBox> {
    Flag::GROUPS
        .iter()
        .flat_map(|group| group.flags.iter().copied())
        .map(|flag| {
            let with_bit = entries
                .iter()
                .filter(|entry| entry.bits & flag.bit() != 0)
                .count();
            let initial = Check::of(with_bit, entries.len());
            FlagBox {
                flag,
                initial,
                current: initial,
                three_state: has_directories || initial == Check::Mixed,
            }
        })
        .collect()
}
