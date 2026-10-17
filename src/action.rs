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

use crate::menu::MenuCommand;
use crate::models::{PanelSide, PanelView};

#[macro_use]
mod keymap;

pub mod bindings;
pub mod chord;
pub mod command_line;
pub mod editor;
pub mod lsp_editor;
pub mod netbox;
pub mod path_env;
pub mod tasks;
pub mod tree;
pub mod vault;
pub mod viewer;
pub mod wsl;

mod commands;
mod scope;

pub use bindings::Bindings;
pub use chord::KeyChord;
pub use commands::{
    DialogCommand, FileCommand, Navigation, PanelCommand, SelectionCommand, ToolCommand,
};
pub use scope::{menu_scope, ActionScope};

/// A single user-triggerable operation in the main dual-panel view.
///
/// Every variant corresponds to exactly one effect applied to the
/// [`App`](crate::models::App); the mapping from keys to actions is
/// [`resolve`]. A key named beside a variant is its default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Select the active panel's file presentation mode.
    PanelView(PanelView),
    /// Typed dialog intent, dispatched by its own owner.
    Dialog(DialogCommand),
    /// Typed selection intent, dispatched by its own owner.
    Selection(SelectionCommand),
    /// Typed navigate intent, dispatched by its own owner.
    Navigate(Navigation),
    /// The Left / Right menu: how the active panel shows its listing.
    Panel(PanelCommand),
    /// The Files menu rows that F-keys alone did not already cover.
    File(FileCommand),
    /// The Commands menu: everything that is not about one file.
    Tool(ToolCommand),
    /// An F9 row outside the panel, file and tool families. Most have no
    /// key and only the menu builds them; Save setup has `Shift+F9`. Either
    /// way the row takes the same road as every other action, so a keyless
    /// row cannot behave differently from a keyed one.
    Menu(MenuCommand),
    /// `Ctrl+C` — leave at once, without asking. The emergency exit stays an
    /// emergency exit: a dialog in front of it would be one keystroke between
    /// the user and a program they have decided to stop.
    ///
    /// Bare `q` is deliberately *not* here: a printable key now starts a
    /// command, and a command line you cannot type `q` into cannot run `quit`,
    /// `qemu` or `queue`.
    Quit,
    /// `Left` / `Right` — make that side the active panel, or, in a
    /// multi-column view, move the cursor one column that way.
    SelectPanel(PanelSide),
    /// `Alt+F1` / `Alt+F2` — open the drive menu for that side.
    DriveMenu(PanelSide),
    /// `F3` — view the selected file.
    View,
    /// `F4` — edit the selected file in the built-in editor.
    Edit,
    /// `Alt+F4` — edit the selected file in the external editor.
    EditExternal,
    /// `F5` — copy to the other panel.
    Copy,
    /// `Shift+F6` — rename the entry where it is.
    Rename,
    /// `F6` — move to the other panel.
    Move,
    /// `F8` — open the delete confirmation dialog.
    DeleteConfirm,
    /// `F11` — Far's Plugin commands list: MCP status, `NetBox`'s stored
    /// sessions and their import.
    PluginCommands,
    /// `Ctrl+O` — toggle console mode (hide panels).
    ToggleConsole,
    /// `Ctrl+R` — reload both panels.
    Refresh,
    /// `:` — open an empty command line.
    CommandLine,
    /// Any printable key — open the command line already holding that
    /// character, the way Far Manager and every shell start a command.
    CommandLineChar(char),
    /// `Alt+<character>` — quick-search: append the character to the query.
    QuickSearch(char),
    /// `Backspace` — drop the last quick-search character.
    QuickSearchBackspace,
    /// `Esc` or any unmapped key — cancel the quick search. Every other
    /// action ends it too; see [`Action::edits_quick_search`].
    QuickSearchClear,
}

impl Action {
    /// Whether this is one of the quick search's own keys — `Alt+<character>`
    /// or `Backspace`. Any other action ends the search before it acts, as
    /// Far's fast find closes on any key that is not part of it.
    #[must_use]
    pub const fn edits_quick_search(self) -> bool {
        matches!(self, Self::QuickSearch(_) | Self::QuickSearchBackspace)
    }
}

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
