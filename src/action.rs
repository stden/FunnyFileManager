//! Input → Action DSL.
//!
//! This module separates *what key was pressed* from *what the app does*. The
//! event loop in [`crate`] resolves a raw [`KeyEvent`] into a typed [`Action`]
//! via [`resolve`], then a dispatcher applies it. Keeping the keymap in one
//! pure, exhaustively-testable function (no terminal, no I/O) mirrors the
//! approach [`crate::command`] already uses for the command line, and makes the
//! full set of main-view bindings reviewable at a glance.
//!
//! Main-view bindings resolve here; [`editor`] declares the built-in editor's
//! commands using the same binding syntax. Modal routing remains owned by
//! [`crate::popup::process_popup_input`].

use crossterm::event::KeyEvent;

use crate::menu::MenuCommand;
use crate::models::{PanelSide, PanelView};

#[macro_use]
mod keymap;
mod menu;

pub mod editor;
pub mod tasks;
pub mod wsl;

mod commands;
mod function_keys;
mod main_view;
mod scope;

pub use commands::{
    DialogCommand, FileCommand, Navigation, PanelCommand, SelectionCommand, ToolCommand,
};
use main_view::resolve_main;
pub use scope::{menu_scope, ActionScope};

/// A single user-triggerable operation in the main dual-panel view.
///
/// Every variant corresponds to exactly one effect applied to the
/// [`App`](crate::models::App); the mapping from keys to actions is [`resolve`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    MenuCommand(MenuCommand),
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
    /// A menu row Far gives no key. `resolve` never returns this — only the
    /// menu builds it — but it takes the same road as every other action, so
    /// a keyless row cannot behave differently from a keyed one.
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

/// Translate a raw key event into the [`Action`] it triggers in the main view.
///
/// This is the single source of truth for main-view key bindings. It is total:
/// any unmapped key resolves to [`Action::QuickSearchClear`], matching the
/// original behaviour where stray keystrokes cancel an in-progress quick search.
///
/// Ordering mirrors the historical dispatch: modifier-qualified bindings
/// (`Alt+F1`, `Shift+F4`, `Ctrl+*`) are checked before their bare counterparts.
#[must_use]
pub const fn resolve(key: KeyEvent) -> Action {
    if let Some(command) = menu::resolve(key) {
        return Action::MenuCommand(command);
    }
    resolve_main(key)
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_far_menu_keys;
