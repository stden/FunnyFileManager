//! Purpose: the keys with a name — Far's spellings, the aliases the file
//! accepts, and the crossterm code each one is.

use std::fmt;

use crossterm::event::KeyCode;

/// A key that is neither a character nor a function key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NamedKey {
    Enter,
    Esc,
    Tab,
    Backspace,
    Ins,
    Del,
    Home,
    End,
    PgUp,
    PgDn,
    Up,
    Down,
    Left,
    Right,
    /// The space bar. A named key rather than the character `' '`, so that
    /// it can be written and read.
    Space,
}

/// Spellings the file accepts besides the canonical ones, in any case.
const ALIASES: [(&str, NamedKey); 5] = [
    ("Escape", NamedKey::Esc),
    ("Insert", NamedKey::Ins),
    ("Delete", NamedKey::Del),
    ("PageUp", NamedKey::PgUp),
    ("PageDown", NamedKey::PgDn),
];

impl NamedKey {
    /// Every named key.
    pub const ALL: [Self; 15] = [
        Self::Enter,
        Self::Esc,
        Self::Tab,
        Self::Backspace,
        Self::Ins,
        Self::Del,
        Self::Home,
        Self::End,
        Self::PgUp,
        Self::PgDn,
        Self::Up,
        Self::Down,
        Self::Left,
        Self::Right,
        Self::Space,
    ];

    /// The canonical spelling: Far's, which FFM's hints already print.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Enter => "Enter",
            Self::Esc => "Esc",
            Self::Tab => "Tab",
            Self::Backspace => "Backspace",
            Self::Ins => "Ins",
            Self::Del => "Del",
            Self::Home => "Home",
            Self::End => "End",
            Self::PgUp => "PgUp",
            Self::PgDn => "PgDn",
            Self::Up => "Up",
            Self::Down => "Down",
            Self::Left => "Left",
            Self::Right => "Right",
            Self::Space => "Space",
        }
    }

    /// The key `text` names — canonical spelling or alias, in any case;
    /// `None` for any other text.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let canonical = Self::ALL
            .into_iter()
            .find(|key| key.name().eq_ignore_ascii_case(text));
        canonical.or_else(|| {
            ALIASES
                .iter()
                .find(|(alias, _)| alias.eq_ignore_ascii_case(text))
                .map(|&(_, key)| key)
        })
    }

    /// The crossterm code a terminal reports for the key.
    #[must_use]
    pub const fn code(self) -> KeyCode {
        match self {
            Self::Enter => KeyCode::Enter,
            Self::Esc => KeyCode::Esc,
            Self::Tab => KeyCode::Tab,
            Self::Backspace => KeyCode::Backspace,
            Self::Ins => KeyCode::Insert,
            Self::Del => KeyCode::Delete,
            Self::Home => KeyCode::Home,
            Self::End => KeyCode::End,
            Self::PgUp => KeyCode::PageUp,
            Self::PgDn => KeyCode::PageDown,
            Self::Up => KeyCode::Up,
            Self::Down => KeyCode::Down,
            Self::Left => KeyCode::Left,
            Self::Right => KeyCode::Right,
            Self::Space => KeyCode::Char(' '),
        }
    }

    /// The named key a crossterm code is; `None` for a character, a
    /// function key, `BackTab` (Shift+Tab, which [`super::KeyChord`] folds)
    /// and every code FFM cannot bind.
    #[must_use]
    pub fn of_code(code: KeyCode) -> Option<Self> {
        Self::ALL.into_iter().find(|key| key.code() == code)
    }
}

impl fmt::Display for NamedKey {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str(self.name())
    }
}
