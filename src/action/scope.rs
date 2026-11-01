//! Purpose: What an action reaches for when it runs — the active panel's
//! entries, or the program at large.
//!
//! A panel that shows something other than local files — `NetBox`'s sessions
//! or a remote host — asks this before letting a key fall through to the
//! file-panel actions: an action that reads or changes the active panel's
//! entries or directory would act on a file list the user cannot see. The
//! matches over actions are exhaustive, so a new action cannot be added
//! without being placed; the one over menu rows falls to the safe side.

use super::{Action, DialogCommand, Navigation};
use crate::menu::MenuCommand;

/// See the module documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionScope {
    /// The entries under the cursor or marked, or the active directory.
    Entries,
    /// The program at large: quitting, the other panel, how a panel is
    /// drawn, menus, the command line, windows of the program's own.
    Program,
}

impl Action {
    /// See [`ActionScope`].
    #[must_use]
    pub const fn scope(self) -> ActionScope {
        match self {
            Self::Menu(command) => menu_scope(command),
            Self::Dialog(command) => dialog_scope(command),
            Self::Navigate(command) => navigation_scope(command),
            Self::Selection(_)
            | Self::File(_)
            | Self::View
            | Self::Edit
            | Self::EditExternal
            | Self::Copy
            | Self::Rename
            | Self::Move
            | Self::DeleteConfirm
            | Self::QuickSearch(_)
            | Self::QuickSearchBackspace => ActionScope::Entries,
            Self::PanelView(_)
            | Self::Panel(_)
            | Self::Tool(_)
            | Self::Quit
            | Self::SelectPanel(_)
            | Self::DriveMenu(_)
            | Self::PluginCommands
            | Self::ToggleConsole
            | Self::Refresh
            | Self::CommandLine
            | Self::CommandLineChar(_)
            | Self::QuickSearchClear => ActionScope::Program,
        }
    }
}

/// A menu row, keyed or not. The rows that only change how the panels are
/// drawn, or open a window of the program's own, are named here; every other
/// row reads or changes the panels' entries.
#[must_use]
pub const fn menu_scope(command: MenuCommand) -> ActionScope {
    match command {
        MenuCommand::Performance
        | MenuCommand::TokenUsage
        | MenuCommand::SystemTempCleanup
        | MenuCommand::LanguageServers
        | MenuCommand::UpdatePackages
        | MenuCommand::McpInstall
        | MenuCommand::ThingsCatalog
        | MenuCommand::Path(_)
        | MenuCommand::Associations
        | MenuCommand::Shortcuts
        | MenuCommand::Devices
        | MenuCommand::Settings(_)
        | MenuCommand::SaveSetup => ActionScope::Program,
        // Open-ended on purpose, and on the safe side: Compare and the user
        // menu act on entries, and a row added later is refused over a
        // `NetBox` panel until it is named above.
        _ => ActionScope::Entries,
    }
}

/// The dialogs that make or measure something in the active directory, or
/// run a user command on its entry, act on the entries.
const fn dialog_scope(command: DialogCommand) -> ActionScope {
    match command {
        DialogCommand::NewFile
        | DialogCommand::UserMenu
        | DialogCommand::Mkdir
        | DialogCommand::Information => ActionScope::Entries,
        DialogCommand::ConfirmQuit
        | DialogCommand::ToggleHelp
        | DialogCommand::WslManagement
        | DialogCommand::Menu => ActionScope::Program,
    }
}

/// Moving the cursor and opening an entry act on the entries; switching or
/// swapping the panels is the program's.
const fn navigation_scope(command: Navigation) -> ActionScope {
    match command {
        Navigation::NavDown
        | Navigation::NavUp
        | Navigation::Open
        | Navigation::First
        | Navigation::Last
        | Navigation::PageUp
        | Navigation::PageDown
        | Navigation::GoRoot => ActionScope::Entries,
        Navigation::SwitchPanel | Navigation::SwapPanels => ActionScope::Program,
    }
}

#[cfg(test)]
mod tests;
