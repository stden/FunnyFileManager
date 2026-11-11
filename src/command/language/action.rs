//! Purpose: The `Action` language: one user-triggerable operation of the main view.

use crate::models::{PanelSide, PanelView};

use super::commands::{
    DialogCommand, FileCommand, Navigation, PanelCommand, SelectionCommand, ToolCommand,
};
use super::menu::MenuCommand;

/// A single user-triggerable operation in the main dual-panel view.
///
/// Every variant corresponds to exactly one effect applied to the
/// [`App`](crate::models::App); the mapping from keys to actions is
/// [`crate::action::resolve`]. A key named beside a variant is its default.
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
    /// Describe this action for the log *without* any text the user typed.
    ///
    /// Both `QuickSearch` and `CommandLineChar` carry a character from the keyboard.
    /// A derived `Debug` would expose that input, so those two are labelled bare.
    #[must_use]
    pub fn log_label(self) -> String {
        match self {
            Self::QuickSearch(_) => "QuickSearch(..)".to_owned(),
            Self::CommandLineChar(_) => "CommandLineChar(..)".to_owned(),
            other => format!("{other:?}"),
        }
    }

    /// Whether this is one of the quick search's own keys — `Alt+<character>`
    /// or `Backspace`. Any other action ends the search before it acts, as
    /// Far's fast find closes on any key that is not part of it.
    #[must_use]
    pub const fn edits_quick_search(self) -> bool {
        matches!(self, Self::QuickSearch(_) | Self::QuickSearchBackspace)
    }
}
