//! Purpose: The F3 viewer's commands, and the one lookup that reads a key
//! as a [`ViewerCommand`] from the keymap's `viewer` bindings.
//!
//! `popup::viewer` scrolls or closes on the command; the quick-view panel
//! takes the scroll commands alone and leaves the rest to the file-panel
//! actions. Which key runs which command is the keymap's to say: the
//! defaults are the rows of `keymap::tables::VIEWER`. Resolving a key has no
//! effect.

use crossterm::event::KeyEvent;

use super::{Action, Bindings, Navigation};

/// What one key asks of the viewer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewerCommand {
    /// Close the viewer.
    Close,
    /// A move of the window over the text, which the quick view shares.
    Scroll(ViewerScroll),
}

/// A move of the viewer's window over its text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewerScroll {
    /// One line up.
    LineUp,
    /// One line down.
    LineDown,
    /// One screen up.
    PageUp,
    /// One screen down.
    PageDown,
    /// The first screen, at the left edge.
    Top,
    /// The last screen, at the left edge.
    Bottom,
    /// One cell to the left.
    Left,
    /// One cell to the right.
    Right,
    /// The wide step to the left.
    LeftWide,
    /// The wide step to the right.
    RightWide,
}

impl ViewerScroll {
    /// The scroll a panels cursor move asks of the quick view when it
    /// arrives without a key — the mouse wheel over the preview. It arrives
    /// as a panel action with no key behind it, so the `viewer` bindings
    /// never reach it: a remap changes the keys, not the wheel. `None` falls
    /// through.
    #[must_use]
    pub const fn from_action(action: Action) -> Option<Self> {
        match action {
            Action::Navigate(Navigation::NavUp) => Some(Self::LineUp),
            Action::Navigate(Navigation::NavDown) => Some(Self::LineDown),
            Action::Navigate(Navigation::PageUp) => Some(Self::PageUp),
            Action::Navigate(Navigation::PageDown) => Some(Self::PageDown),
            Action::Navigate(Navigation::First) => Some(Self::Top),
            Action::Navigate(Navigation::Last) => Some(Self::Bottom),
            _ => None,
        }
    }
}

/// What `key` asks of the viewer under `bindings`.
///
/// The keymap's `viewer` context gives the command of exactly that chord, so
/// Shift+F3 is not F3. `None` for every other key: the context has no
/// fallback of its own, so the viewer swallows the key and the quick view
/// hands it to the panels.
#[must_use]
pub fn resolve(bindings: &Bindings<ViewerCommand>, key: KeyEvent) -> Option<ViewerCommand> {
    bindings.resolve(key)
}

#[cfg(test)]
mod tests;
