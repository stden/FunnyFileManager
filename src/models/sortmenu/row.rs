//! Purpose: One row of the sort modes menu — a mode or the order switch — with
//! its label, its `Ctrl+F` shortcut and the letter that picks it.

use super::SortMode;

/// One row of the menu: a mode, or the order switch under the rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortRow {
    Mode(SortMode),
    /// Far's `-` and `*`: the same mode, run the other way.
    Reverse,
}

impl SortRow {
    /// In the order Far's help lists them, the order switch last.
    pub const ALL: [Self; 10] = [
        Self::Mode(SortMode::Name),
        Self::Mode(SortMode::Extension),
        Self::Mode(SortMode::ModificationTime),
        Self::Mode(SortMode::Size),
        Self::Mode(SortMode::Unsorted),
        Self::Mode(SortMode::CreationTime),
        Self::Mode(SortMode::AccessTime),
        Self::Mode(SortMode::Descriptions),
        Self::Mode(SortMode::Owner),
        Self::Reverse,
    ];

    /// The function key of the first mode row; the rest follow in menu order.
    const FIRST_KEY: usize = 3;

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Mode(mode) => mode.label(),
            Self::Reverse => "Reverse order",
        }
    }

    /// The key the row answers to outside the menu: `Ctrl+F3` … `Ctrl+F11`
    /// for the modes, and `*` — the menu's own order flip — for the switch.
    #[must_use]
    pub fn shortcut(self) -> String {
        match self {
            Self::Mode(mode) => {
                let index = SortMode::ALL.iter().position(|m| *m == mode);
                format!(
                    "Ctrl+F{}",
                    index.map_or(Self::FIRST_KEY, |i| i + Self::FIRST_KEY)
                )
            }
            Self::Reverse => "*".to_owned(),
        }
    }

    /// The letter that picks the row from the keyboard — the mode's panel
    /// header letter, so one letter means one mode everywhere.
    #[must_use]
    pub const fn hotkey(self) -> char {
        match self {
            Self::Mode(mode) => mode.letter(),
            Self::Reverse => 'r',
        }
    }

    /// Far's own mnemonic for the extension row — "e&Xtension" — kept as an
    /// alias, since a Far hand reaches for it before it reads the label.
    const FAR_EXTENSION_MNEMONIC: char = 'x';

    /// The row a typed letter names, in either case.
    #[must_use]
    pub fn by_hotkey(letter: char) -> Option<Self> {
        let wanted = letter.to_ascii_lowercase();
        if wanted == Self::FAR_EXTENSION_MNEMONIC {
            return Some(Self::Mode(SortMode::Extension));
        }
        Self::ALL.into_iter().find(|row| row.hotkey() == wanted)
    }

    /// The mode row `Ctrl+F<number>` names — the inverse of [`Self::shortcut`],
    /// so the key works inside the menu as it does outside it.
    #[must_use]
    pub fn by_function_key(number: u8) -> Option<Self> {
        let index = usize::from(number).checked_sub(Self::FIRST_KEY)?;
        SortMode::ALL.get(index).copied().map(Self::Mode)
    }
}
