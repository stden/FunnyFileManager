//! Purpose: how the tables of default keys write a chord — typed, in
//! constants, with no text to parse and so no error to handle.
//!
//! `f(3)`, `ch('+')`, `named(Ins)`, and a modifier around any of them:
//! `shift(f(4))`, `ctrl(ch('r'))`, `ctrl(alt(ch('w')))`. The tables' tests
//! read every chord back through its text, so a chord written against
//! `KeyChord`'s invariants (a capital under Ctrl, Shift with a character)
//! fails there.

use crate::action::chord::{FunctionKey, Key, Modifier, Modifiers, NamedKey};
use crate::action::KeyChord;

/// The key alone.
const fn bare(key: Key) -> KeyChord {
    KeyChord {
        key,
        modifiers: Modifiers::NONE,
    }
}

const fn holding(chord: KeyChord, modifier: Modifier) -> KeyChord {
    KeyChord {
        key: chord.key,
        modifiers: chord.modifiers.with(modifier),
    }
}

/// The character's key: `ch('+')`, `ch('K')` for Shift and `k`.
pub(super) const fn ch(character: char) -> KeyChord {
    bare(Key::Char(character))
}

/// F`number`.
pub(super) const fn f(number: u8) -> KeyChord {
    bare(Key::F(FunctionKey::saturating(number)))
}

/// A key with a name: `named(Enter)`.
pub(super) const fn named(key: NamedKey) -> KeyChord {
    bare(Key::Named(key))
}

/// The chord with Ctrl held.
pub(super) const fn ctrl(chord: KeyChord) -> KeyChord {
    holding(chord, Modifier::Ctrl)
}

/// The chord with Alt held.
pub(super) const fn alt(chord: KeyChord) -> KeyChord {
    holding(chord, Modifier::Alt)
}

/// The chord with Shift held.
pub(super) const fn shift(chord: KeyChord) -> KeyChord {
    holding(chord, Modifier::Shift)
}

#[cfg(test)]
mod tests;
