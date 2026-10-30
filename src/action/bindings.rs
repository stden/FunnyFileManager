//! Purpose: `Bindings`, the keys of one screen in force — the command each
//! chord runs, and each command's chords in the order they were given.
//!
//! The one lookup every resolver makes, and it is exact: Shift+F5 is not F5.
//! A chord has one command by construction, because nothing but
//! [`Bindings::bind`] puts a chord in, and it moves a chord that is already
//! held. Everything here is pure: no state beyond the value, no I/O.

use crossterm::event::KeyEvent;

use super::chord::Key;
use super::KeyChord;

/// The keys of one context (`panels`, `editor`, …), each with its command.
///
/// A context holds a hundred chords at most, so the lookup walks them
/// rather than keep an index that could disagree with the order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bindings<C> {
    /// Every chord in force beside its command, in the order they were
    /// bound. No chord appears twice.
    held: Vec<(KeyChord, C)>,
}

impl<C> Default for Bindings<C> {
    /// No key at all.
    fn default() -> Self {
        Self { held: Vec::new() }
    }
}

impl<C: Copy + PartialEq> Bindings<C> {
    /// The command `chord` runs; `None` for a chord nothing is bound to,
    /// which the context's fallback answers. Exactly this chord: a modifier
    /// more or less is another key.
    #[must_use]
    pub fn command(&self, chord: KeyChord) -> Option<C> {
        self.held
            .iter()
            .find(|(held, _)| *held == chord)
            .map(|&(_, command)| command)
    }

    /// The command the key runs: the chord it means on this platform
    /// ([`KeyChord::from_event`]), looked up by [`Self::command`], falling
    /// back to the physical `QWERTY` chord when a Russian `ЙЦУКЕН` layout
    /// reports a Cyrillic letter under `Ctrl` or `Alt`. `None` for a key no
    /// chord names (`CapsLock`, a media key) and for a chord nothing is bound
    /// to.
    #[must_use]
    pub fn resolve(&self, key: KeyEvent) -> Option<C> {
        let chord = KeyChord::from_event(key)?;
        self.command(chord)
            .or_else(|| qwerty_chord(chord).and_then(|fallback| self.command(fallback)))
    }

    /// The chords that run `command`, in the order they were bound; the
    /// first is the one a menu or a key bar prints. None for a command
    /// without a key.
    pub fn keys(&self, command: C) -> impl Iterator<Item = KeyChord> + '_ {
        self.held
            .iter()
            .filter(move |(_, held)| *held == command)
            .map(|&(chord, _)| chord)
    }

    /// Every chord in force with its command, in the order they were bound.
    pub fn chords(&self) -> impl Iterator<Item = (KeyChord, C)> + '_ {
        self.held.iter().copied()
    }

    /// Gives `chord` to `command` and returns the command that held it
    /// until now, which has lost it: a chord runs one command. A chord the
    /// command holds already stays where it is among its keys.
    pub fn bind(&mut self, command: C, chord: KeyChord) -> Option<C> {
        let before = self.command(chord);
        if before == Some(command) {
            return before;
        }
        self.held.retain(|(held, _)| *held != chord);
        self.held.push((chord, command));
        before
    }
}

/// Physical `QWERTY` key for each Russian `ЙЦУКЕН` letter.
const RUSSIAN_KEYS: [(char, char); 33] = [
    ('ё', '`'),
    ('й', 'q'),
    ('ц', 'w'),
    ('у', 'e'),
    ('к', 'r'),
    ('е', 't'),
    ('н', 'y'),
    ('г', 'u'),
    ('ш', 'i'),
    ('щ', 'o'),
    ('з', 'p'),
    ('х', '['),
    ('ъ', ']'),
    ('ф', 'a'),
    ('ы', 's'),
    ('в', 'd'),
    ('а', 'f'),
    ('п', 'g'),
    ('р', 'h'),
    ('о', 'j'),
    ('л', 'k'),
    ('д', 'l'),
    ('ж', ';'),
    ('э', '\''),
    ('я', 'z'),
    ('ч', 'x'),
    ('с', 'c'),
    ('м', 'v'),
    ('и', 'b'),
    ('т', 'n'),
    ('ь', 'm'),
    ('б', ','),
    ('ю', '.'),
];

/// The physical `QWERTY` chord for a `Ctrl` or `Alt` chord typed on a Russian
/// `ЙЦУКЕН` keyboard layout (`Ctrl+Щ` → `Ctrl+O`, `Ctrl+С` → `Ctrl+C`,
/// `Ctrl+М` → `Ctrl+V`).
fn qwerty_chord(chord: KeyChord) -> Option<KeyChord> {
    if !chord.modifiers.ctrl_or_alt() {
        return None;
    }
    let Key::Char(ch) = chord.key else {
        return None;
    };
    let (_, latin) = RUSSIAN_KEYS
        .into_iter()
        .find(|&(cyrillic, _)| cyrillic == ch)?;
    Some(KeyChord {
        key: Key::Char(latin),
        modifiers: chord.modifiers,
    })
}

#[cfg(test)]
mod tests;
