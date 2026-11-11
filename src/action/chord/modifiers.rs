//! Purpose: the three modifier keys a binding can name, and a set of them.

use std::fmt;

/// One of the modifier keys FFM binds. Super, Hyper and Meta are not among
/// them: a desktop that passes them through must not make a key dead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Modifier {
    Ctrl,
    Alt,
    Shift,
}

impl Modifier {
    /// Every modifier, in the order the canonical text writes them.
    pub const ALL: [Self; 3] = [Self::Ctrl, Self::Alt, Self::Shift];

    /// The spelling in the file and on screen.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Ctrl => "Ctrl",
            Self::Alt => "Alt",
            Self::Shift => "Shift",
        }
    }

    /// The modifier `text` names, in any case; `None` for any other text.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|modifier| modifier.name().eq_ignore_ascii_case(text))
    }
}

impl fmt::Display for Modifier {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str(self.name())
    }
}

/// The modifiers held with a key: any subset of Ctrl, Alt and Shift.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Modifiers {
    ctrl: bool,
    alt: bool,
    shift: bool,
}

impl Modifiers {
    /// No modifier held.
    pub const NONE: Self = Self {
        ctrl: false,
        alt: false,
        shift: false,
    };
    pub const CTRL: Self = Self::NONE.with(Modifier::Ctrl);
    pub const ALT: Self = Self::NONE.with(Modifier::Alt);
    pub const SHIFT: Self = Self::NONE.with(Modifier::Shift);

    /// This set with `modifier` held.
    #[must_use]
    pub const fn with(self, modifier: Modifier) -> Self {
        match modifier {
            Modifier::Ctrl => Self { ctrl: true, ..self },
            Modifier::Alt => Self { alt: true, ..self },
            Modifier::Shift => Self {
                shift: true,
                ..self
            },
        }
    }

    /// This set with `modifier` released.
    #[must_use]
    pub const fn without(self, modifier: Modifier) -> Self {
        match modifier {
            Modifier::Ctrl => Self {
                ctrl: false,
                ..self
            },
            Modifier::Alt => Self { alt: false, ..self },
            Modifier::Shift => Self {
                shift: false,
                ..self
            },
        }
    }

    #[must_use]
    pub const fn contains(self, modifier: Modifier) -> bool {
        match modifier {
            Modifier::Ctrl => self.ctrl,
            Modifier::Alt => self.alt,
            Modifier::Shift => self.shift,
        }
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        !self.ctrl && !self.alt && !self.shift
    }

    /// Whether Ctrl or Alt is held — the modifiers that make a character a
    /// command rather than text, and under which Shift is written out.
    #[must_use]
    pub const fn ctrl_or_alt(self) -> bool {
        self.ctrl || self.alt
    }

    /// The modifiers held, in the order the canonical text writes them.
    pub fn iter(self) -> impl Iterator<Item = Modifier> {
        Modifier::ALL
            .into_iter()
            .filter(move |&modifier| self.contains(modifier))
    }
}
