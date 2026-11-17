//! Purpose: the chord a crossterm `KeyEvent` means, and the event a chord
//! stands for — one normalization for every terminal and platform.
//!
//! Read from crossterm 0.29's parsers. Unix legacy bytes and the kitty
//! protocol put Shift into a character (`K`, `+`) and, under Ctrl, kitty
//! reports the shifted letter upper-case with the SHIFT flag dropped; the
//! Windows console keeps the SHIFT flag beside the character Shift produced,
//! upper-cases a letter under `CapsLock` instead, and under Ctrl reports the
//! unshifted key. The rules are a [`Platform`] value rather than two
//! `#[cfg]` functions, so that both run in every build's tests.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{lowercase, uppercase, FunctionKey, Key, KeyChord, Modifier, Modifiers, NamedKey};

/// Whose key reports are being read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    /// Linux and macOS: VT bytes, or the kitty protocol where FFM pushed it.
    Unix,
    /// The Windows console, which reports key codes and `CapsLock`'s case.
    Windows,
}

impl Platform {
    /// The platform this build runs on.
    pub const CURRENT: Self = if cfg!(windows) {
        Self::Windows
    } else {
        Self::Unix
    };

    /// The chord `key` means on this platform; `None` for a key FFM cannot
    /// bind (`CapsLock`, `NumLock`, a media key, a bare modifier, `Null`, F0,
    /// F25 and above), which a context's fallback handles.
    ///
    /// Super, Hyper and Meta are dropped. `BackTab` is Tab with Shift. A
    /// letter under Ctrl or Alt is lower-cased and Shift comes from the
    /// SHIFT flag — on Unix also from an upper-case letter, which is kitty's
    /// shifted key; on Windows an upper-case letter without SHIFT is
    /// `CapsLock` and means no Shift. Any other character, and Space, loses
    /// its SHIFT flag unless Ctrl is held, because Shift is already in the
    /// character: Unix sends Alt+Shift+= as Alt+`'+'` with no flag, Windows
    /// as Alt+`'+'` with SHIFT, and both mean `Alt++`. Under Ctrl the flag
    /// stays, because the Windows console then reports the unshifted key
    /// (`'='` + CONTROL + SHIFT), which is [`super::Delivery::ShiftedSymbol`]'s
    /// case. The event's kind is not read: the event loop passes presses only.
    #[must_use]
    pub fn chord(self, key: KeyEvent) -> Option<KeyChord> {
        let held = held_modifiers(key.modifiers);
        match key.code {
            KeyCode::Char(character) => Some(self.character(character, held)),
            KeyCode::BackTab => Some(KeyChord {
                key: Key::Named(NamedKey::Tab),
                modifiers: held.with(Modifier::Shift),
            }),
            KeyCode::F(number) => FunctionKey::new(number).map(|function_key| KeyChord {
                key: Key::F(function_key),
                modifiers: held,
            }),
            code => NamedKey::of_code(code).map(|named| KeyChord {
                key: Key::Named(named),
                modifiers: held,
            }),
        }
    }

    /// A character key as this platform reports it.
    fn character(self, character: char, held: Modifiers) -> KeyChord {
        let key = match character {
            ' ' => Key::Named(NamedKey::Space),
            other => Key::Char(other),
        };
        match lowercase(character) {
            Some(lower) if held.ctrl_or_alt() => self.letter(lower, character, held),
            // Shift is in the character itself (`K`, `+`, a space) unless
            // Ctrl is held: only then does the Windows console report the
            // unshifted key with the flag.
            _ if held.contains(Modifier::Ctrl) => KeyChord {
                key,
                modifiers: held,
            },
            _ => KeyChord {
                key,
                modifiers: held.without(Modifier::Shift),
            },
        }
    }

    /// A letter under Ctrl or Alt: lower-case, with Shift from the flag and,
    /// on Unix, from kitty's upper-case letter, which on Windows is `CapsLock`.
    fn letter(self, lower: char, typed: char, held: Modifiers) -> KeyChord {
        let shifted = held.contains(Modifier::Shift) || (self == Self::Unix && lower != typed);
        KeyChord {
            key: Key::Char(lower),
            modifiers: if shifted {
                held.with(Modifier::Shift)
            } else {
                held
            },
        }
    }
}

/// Ctrl, Alt and Shift from the event's flags; Super, Hyper and Meta are
/// not bindable and are dropped.
fn held_modifiers(flags: KeyModifiers) -> Modifiers {
    FLAGS
        .into_iter()
        .filter(|&(flag, _)| flags.contains(flag))
        .fold(Modifiers::NONE, |held, (_, modifier)| held.with(modifier))
}

/// Each bindable modifier and its crossterm flag.
const FLAGS: [(KeyModifiers, Modifier); 3] = [
    (KeyModifiers::CONTROL, Modifier::Ctrl),
    (KeyModifiers::ALT, Modifier::Alt),
    (KeyModifiers::SHIFT, Modifier::Shift),
];

impl KeyChord {
    /// The chord `key` means on the platform this build runs on; see
    /// [`Platform::chord`].
    #[must_use]
    pub fn from_event(key: KeyEvent) -> Option<Self> {
        Platform::CURRENT.chord(key)
    }

    /// The canonical event of the chord, which [`Self::from_event`] turns
    /// back into it on every platform: a legacy Unix terminal's event where
    /// that reads the same on both — an upper-case character carries SHIFT,
    /// a shifted letter under Alt is upper-case with the SHIFT flag (the
    /// Windows console's event too), Shift+Tab is `BackTab`. A shifted
    /// letter under Ctrl has no such event (legacy bytes lose the Shift,
    /// kitty upper-cases the letter and drops the flag, Windows keeps both),
    /// so it goes lower-case with the SHIFT flag: read as itself on both
    /// platforms, and the lower-case letter the old resolvers match.
    #[must_use]
    pub fn to_event(self) -> KeyEvent {
        let mut flags = FLAGS
            .into_iter()
            .filter(|&(_, modifier)| self.modifiers.contains(modifier))
            .fold(KeyModifiers::empty(), |flags, (flag, _)| flags | flag);
        let code = match self.key {
            Key::Char(character) => {
                if !self.modifiers.ctrl_or_alt() && character.is_uppercase() {
                    flags |= KeyModifiers::SHIFT;
                }
                KeyCode::Char(self.typed(character))
            }
            Key::F(function_key) => KeyCode::F(function_key.number()),
            Key::Named(NamedKey::Tab) if self.modifiers.contains(Modifier::Shift) => {
                KeyCode::BackTab
            }
            Key::Named(named) => named.code(),
        };
        KeyEvent::new(code, flags)
    }

    /// The character the terminal types for the chord's: upper-case for a
    /// shifted letter under Alt alone, the chord's own otherwise.
    fn typed(self, character: char) -> char {
        let alt_shift =
            self.modifiers.contains(Modifier::Shift) && !self.modifiers.contains(Modifier::Ctrl);
        if alt_shift {
            uppercase(character).unwrap_or(character)
        } else {
            character
        }
    }
}
