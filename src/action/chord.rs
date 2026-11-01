//! Purpose: `KeyChord`, the one value every key table is keyed by — a typed
//! key with the Ctrl, Alt and Shift held with it — and its text form.
//!
//! A crossterm `KeyEvent` differs between Linux, macOS, Windows and the kitty
//! protocol for the same physical keys (Shift folded into a character or
//! not, letters upper-cased or not). [`KeyChord::from_event`] normalizes them
//! into one value, so a binding written once in `ffm.yml` matches on every
//! terminal; [`crate::action::chord::Delivery`] says where it cannot.
//! Everything here is pure: no state, no I/O.

use std::fmt;

mod delivery;
mod event;
mod modifiers;
mod named;
mod text;

pub use delivery::{Delivery, Taker};
pub use event::Platform;
pub use modifiers::{Modifier, Modifiers};
pub use named::NamedKey;
pub use text::KeyError;

/// A key together with the modifiers held with it: `Ctrl+Shift+F3`, `K`,
/// `Alt++`.
///
/// Two invariants hold for every chord that [`KeyChord::from_event`] or the
/// text syntax produces: Shift is never held with a character or Space
/// unless Ctrl is too, or Alt with a letter, because Shift is otherwise
/// part of the character itself (`K`, `+`); and a letter under Ctrl or Alt
/// is lower-case, so `Ctrl+r` and `Ctrl+R` are one chord.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeyChord {
    pub key: Key,
    pub modifiers: Modifiers,
}

/// What was pressed, apart from the modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Key {
    /// A printable character other than space, as the keyboard layout types
    /// it: `k`, `K`, `+`, `ж`.
    Char(char),
    /// F1 to F24.
    F(FunctionKey),
    /// Enter, Esc, the arrows, Space and the other keys with a name.
    Named(NamedKey),
}

impl fmt::Display for Key {
    /// The key alone: a letter keeps its case (a chord upper-cases it under
    /// Ctrl or Alt).
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Char(character) => write!(out, "{character}"),
            Self::F(function_key) => write!(out, "{function_key}"),
            Self::Named(named) => write!(out, "{named}"),
        }
    }
}

/// A function key, F1 to F24.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FunctionKey(u8);

impl FunctionKey {
    /// The highest function key a terminal reports.
    pub const LAST: u8 = 24;

    /// F`number`; `None` outside F1 to F24.
    #[must_use]
    pub const fn new(number: u8) -> Option<Self> {
        if number >= 1 && number <= Self::LAST {
            Some(Self(number))
        } else {
            None
        }
    }

    /// F`number` held within F1 to F24: F0 is F1, F25 is F24. For the
    /// tables of default keys, which are constants and have no error path;
    /// their tests read every key back.
    #[must_use]
    pub const fn saturating(number: u8) -> Self {
        if number < 1 {
            Self(1)
        } else if number > Self::LAST {
            Self(Self::LAST)
        } else {
            Self(number)
        }
    }

    /// 3 for F3.
    #[must_use]
    pub const fn number(self) -> u8 {
        self.0
    }
}

impl fmt::Display for FunctionKey {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(out, "F{}", self.0)
    }
}

/// The lower-case form of a cased letter; `None` for a digit, a symbol, a
/// caseless script (`中`) or a letter whose lower case is more than one
/// character.
fn lowercase(character: char) -> Option<char> {
    let is_letter = character.is_lowercase() || character.is_uppercase();
    is_letter
        .then(|| single(character.to_lowercase()))
        .flatten()
}

/// The upper-case form of a cased letter; `None` as [`lowercase`] says, and
/// for `ß`, whose upper case is `SS`.
fn uppercase(character: char) -> Option<char> {
    let is_letter = character.is_lowercase() || character.is_uppercase();
    is_letter
        .then(|| single(character.to_uppercase()))
        .flatten()
}

/// The one character a case mapping produced, if it produced exactly one.
fn single(mut mapped: impl Iterator<Item = char>) -> Option<char> {
    let first = mapped.next()?;
    mapped.next().is_none().then_some(first)
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_delivery;
#[cfg(test)]
mod tests_event;
#[cfg(test)]
mod tests_event_named;
#[cfg(test)]
mod tests_event_symbols;
#[cfg(test)]
mod tests_round_trip;
#[cfg(test)]
mod tests_syntax;
#[cfg(test)]
mod tests_syntax_shift;
