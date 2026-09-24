//! Purpose: scopes of what no key reaches directly — F9 menu rows, dialogs, panel moves and the program's own actions.

use super::*;

/// Menu rows that act on entries.
const ENTRY_ROWS: [MenuCommand; 10] = [
    MenuCommand::Link,
    MenuCommand::Wipe,
    MenuCommand::Archive,
    MenuCommand::Extract,
    MenuCommand::ArchiveCommands,
    MenuCommand::Attributes,
    MenuCommand::ApplyCommand,
    MenuCommand::Describe,
    MenuCommand::Compare,
    MenuCommand::UserMenu,
];

/// Menu rows that belong to the program.
const PROGRAM_ROWS: [MenuCommand; 24] = [
    MenuCommand::Performance,
    MenuCommand::LanguageServers,
    MenuCommand::UpdatePackages,
    MenuCommand::McpInstall,
    MenuCommand::Tree,
    MenuCommand::QuickView,
    MenuCommand::Sort,
    MenuCommand::LongNames,
    MenuCommand::TogglePanel(PanelSide::Left),
    MenuCommand::FindFile,
    MenuCommand::History,
    MenuCommand::VideoMode,
    MenuCommand::FindFolder,
    MenuCommand::ViewHistory,
    MenuCommand::FolderHistory,
    MenuCommand::Associations,
    MenuCommand::Shortcuts,
    MenuCommand::Filter,
    MenuCommand::Screens,
    MenuCommand::Tasks,
    MenuCommand::Devices,
    MenuCommand::Settings(SettingsPage::Colors),
    MenuCommand::SaveSetup,
    MenuCommand::Path(crate::path_env::Scope::User),
];

#[test]
fn a_menu_row_is_placed_whether_or_not_it_has_a_key() {
    for command in ENTRY_ROWS {
        assert_eq!(menu_scope(command), ActionScope::Entries, "{command:?}");
        assert_eq!(Action::Menu(command).scope(), ActionScope::Entries);
    }
    for command in PROGRAM_ROWS {
        assert_eq!(menu_scope(command), ActionScope::Program, "{command:?}");
        assert_eq!(Action::MenuCommand(command).scope(), ActionScope::Program);
    }
}

const ENTRY_DIALOGS: [DialogCommand; 4] = [
    DialogCommand::NewFile,
    DialogCommand::UserMenu,
    DialogCommand::Mkdir,
    DialogCommand::Information,
];

const PROGRAM_DIALOGS: [DialogCommand; 4] = [
    DialogCommand::ConfirmQuit,
    DialogCommand::ToggleHelp,
    DialogCommand::WslManagement,
    DialogCommand::Menu,
];

#[test]
fn dialogs_are_placed_by_what_they_touch() {
    for dialog in ENTRY_DIALOGS {
        assert_eq!(
            Action::Dialog(dialog).scope(),
            ActionScope::Entries,
            "{dialog:?}"
        );
    }
    for dialog in PROGRAM_DIALOGS {
        assert_eq!(
            Action::Dialog(dialog).scope(),
            ActionScope::Program,
            "{dialog:?}"
        );
    }
}

#[test]
fn moves_and_the_programs_own_actions_are_placed() {
    for moving in [Navigation::SwitchPanel, Navigation::SwapPanels] {
        assert_eq!(Action::Navigate(moving).scope(), ActionScope::Program);
    }
    assert_eq!(
        Action::Navigate(Navigation::GoRoot).scope(),
        ActionScope::Entries
    );
    assert_eq!(Action::PluginCommands.scope(), ActionScope::Program);
    assert_eq!(Action::CommandLine.scope(), ActionScope::Program);
    assert_eq!(Action::CommandLineChar('x').scope(), ActionScope::Program);
}
