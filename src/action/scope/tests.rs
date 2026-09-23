//! Unit tests: every key that would act on the active panel's entries is
//! placed there, the program's keys are the program's, and a menu row not
//! yet placed falls to the safe side.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{menu_scope, ActionScope};
use crate::action::{resolve, Action, DialogCommand, Navigation};
use crate::menu::{MenuCommand, SettingsPage};
use crate::models::PanelSide;

const fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

const fn with(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

/// Keys that act on the active panel's entries.
const ENTRY_KEYS: [KeyEvent; 35] = [
    key(KeyCode::F(2)),
    key(KeyCode::F(3)),
    key(KeyCode::F(4)),
    key(KeyCode::F(5)),
    key(KeyCode::F(6)),
    key(KeyCode::F(7)),
    key(KeyCode::F(8)),
    key(KeyCode::Enter),
    key(KeyCode::Up),
    key(KeyCode::Down),
    key(KeyCode::Home),
    key(KeyCode::End),
    key(KeyCode::PageUp),
    key(KeyCode::PageDown),
    key(KeyCode::Insert),
    key(KeyCode::Char(' ')),
    key(KeyCode::Char('+')),
    key(KeyCode::Char('-')),
    key(KeyCode::Char('*')),
    key(KeyCode::Backspace),
    with(KeyCode::F(1), KeyModifiers::SHIFT),
    with(KeyCode::F(2), KeyModifiers::SHIFT),
    with(KeyCode::F(3), KeyModifiers::SHIFT),
    with(KeyCode::F(4), KeyModifiers::SHIFT),
    with(KeyCode::F(6), KeyModifiers::SHIFT),
    with(KeyCode::F(4), KeyModifiers::ALT),
    with(KeyCode::F(6), KeyModifiers::ALT),
    with(KeyCode::Delete, KeyModifiers::ALT),
    with(KeyCode::Char('a'), KeyModifiers::CONTROL),
    with(KeyCode::Char('g'), KeyModifiers::CONTROL),
    with(KeyCode::Char('z'), KeyModifiers::CONTROL),
    with(KeyCode::Char('l'), KeyModifiers::CONTROL),
    with(KeyCode::Char('m'), KeyModifiers::CONTROL),
    with(KeyCode::Char('\\'), KeyModifiers::CONTROL),
    with(KeyCode::Char('x'), KeyModifiers::ALT),
];

/// Keys that belong to the program at large.
const PROGRAM_KEYS: [KeyEvent; 34] = [
    key(KeyCode::Tab),
    key(KeyCode::Esc),
    key(KeyCode::Left),
    key(KeyCode::Right),
    key(KeyCode::F(1)),
    key(KeyCode::F(9)),
    key(KeyCode::F(10)),
    key(KeyCode::F(11)),
    key(KeyCode::F(12)),
    key(KeyCode::Char(':')),
    key(KeyCode::Char('q')),
    with(KeyCode::F(1), KeyModifiers::ALT),
    with(KeyCode::F(2), KeyModifiers::ALT),
    with(KeyCode::F(7), KeyModifiers::ALT),
    with(KeyCode::F(8), KeyModifiers::ALT),
    with(KeyCode::F(9), KeyModifiers::ALT),
    with(KeyCode::F(10), KeyModifiers::ALT),
    with(KeyCode::F(11), KeyModifiers::ALT),
    with(KeyCode::F(12), KeyModifiers::ALT),
    with(KeyCode::F(9), KeyModifiers::SHIFT),
    with(KeyCode::F(1), KeyModifiers::CONTROL),
    with(KeyCode::F(3), KeyModifiers::CONTROL),
    with(KeyCode::F(12), KeyModifiers::CONTROL),
    with(KeyCode::Char('c'), KeyModifiers::CONTROL),
    with(KeyCode::Char('i'), KeyModifiers::CONTROL),
    with(KeyCode::Char('n'), KeyModifiers::CONTROL),
    with(KeyCode::Char('o'), KeyModifiers::CONTROL),
    with(KeyCode::Char('q'), KeyModifiers::CONTROL),
    with(KeyCode::Char('r'), KeyModifiers::CONTROL),
    with(KeyCode::Char('t'), KeyModifiers::CONTROL),
    with(KeyCode::Char('u'), KeyModifiers::CONTROL),
    with(KeyCode::Char('w'), KeyModifiers::CONTROL),
    with(KeyCode::Char('1'), KeyModifiers::CONTROL),
    with(
        KeyCode::Char('w'),
        KeyModifiers::CONTROL.union(KeyModifiers::ALT),
    ),
];

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
const PROGRAM_ROWS: [MenuCommand; 23] = [
    MenuCommand::Performance,
    MenuCommand::LanguageServers,
    MenuCommand::UpdatePackages,
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
fn keys_that_act_on_entries_are_scoped_to_them() {
    for event in ENTRY_KEYS {
        assert_eq!(resolve(event).scope(), ActionScope::Entries, "{event:?}");
    }
}

#[test]
fn the_programs_keys_are_scoped_to_the_program() {
    for event in PROGRAM_KEYS {
        assert_eq!(resolve(event).scope(), ActionScope::Program, "{event:?}");
    }
}

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
