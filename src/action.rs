//! Input → Action.
//!
//! This module separates *what key was pressed* from *what the app does*. The
//! event loop in [`crate`] resolves a raw [`KeyEvent`] into a typed [`Action`]
//! via [`resolve`], then a dispatcher applies it. Which key runs which action
//! is the `panels` context of the keymap ([`crate::keymap`]), handed in as
//! [`Bindings`]; the lookup is exact, and a key nothing is bound to gets the
//! panels' fixed fallback. Resolution stays pure: no terminal, no I/O.
//!
//! Main-view keys resolve here; every other screen has its module below.
//! Modal routing remains owned by [`crate::popup::process_popup_input`].

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[macro_use]
mod keymap;

pub mod bindings;
pub mod chord;
pub mod command_line;
pub mod database;
pub mod editor;
pub mod lsp_editor;
pub mod netbox;
pub mod path_env;
pub mod tasks;
pub mod tree;
pub mod vault;
pub mod viewer;
pub mod wsl;

mod scope;

pub use crate::command::language::{
    Action, DialogCommand, FileCommand, Navigation, PanelCommand, SelectionCommand, ToolCommand,
};
pub use bindings::Bindings;
pub use chord::KeyChord;
pub use scope::{menu_scope, ActionScope};

/// Whether the F9 menu's own keys take `key`: the chord it means in `bindings`
/// runs a menu command, a Left / Right, file, tool or F9 row.
///
/// Console mode runs a menu command only when the menu takes the key. The
/// lookup is exact, so a chord with extra modifiers is another key and runs
/// nothing behind the transcript.
#[must_use]
pub fn is_menu_key(bindings: &Bindings<Action>, key: KeyEvent) -> bool {
    matches!(
        bindings.resolve(key),
        Some(Action::Panel(_) | Action::File(_) | Action::Tool(_) | Action::Menu(_))
    )
}

/// The [`Action`] a key runs in the main view, under `bindings` — the
/// `panels` context of the keymap.
///
/// The key is looked up as the chord it means, with exactly its modifiers:
/// `Shift+F5` is not `F5`. It is total: a key nothing is bound to gets the
/// panels' fixed fallback.
#[must_use]
pub fn resolve(bindings: &Bindings<Action>, key: KeyEvent) -> Action {
    bindings.resolve(key).unwrap_or_else(|| fallback(key))
}

/// What the panels do with a key nothing is bound to. The two families are
/// rules over every character rather than keys, so no binding names them:
/// Alt with a character searches for it, and any other character opens the
/// command line holding it. They read the event itself, not its chord, so
/// the text is what was typed, `CapsLock` included. Ctrl with a character
/// must not become command text, and like every other stray key it only
/// ends the quick search.
const fn fallback(key: KeyEvent) -> Action {
    let KeyCode::Char(typed) = key.code else {
        return Action::QuickSearchClear;
    };
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        Action::QuickSearchClear
    } else if key.modifiers.contains(KeyModifiers::ALT) {
        Action::QuickSearch(typed)
    } else {
        Action::CommandLineChar(typed)
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_far_menu_keys;
