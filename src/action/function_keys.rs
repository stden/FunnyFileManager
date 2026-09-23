//! Purpose: the F-key tables of the main view, one per modifier, each falling through to the bare key.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::models::{PanelSide, SortMode};

use super::{Action, FileCommand, PanelCommand, ToolCommand};

/// A function key, by its modifier. Each modified table falls through to the
/// bare key for combinations it does not name, which is what `has` did
/// before the tables were split: `Ctrl+F5` still copies.
pub(super) const fn resolve_function_key(key: KeyEvent) -> Action {
    if key.modifiers.contains(KeyModifiers::ALT) {
        return resolve_alt_function_key(key);
    }
    if key.modifiers.contains(KeyModifiers::SHIFT) {
        return resolve_shift_function_key(key);
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return resolve_ctrl_function_key(key);
    }
    resolve_bare_function_key(key)
}

const fn resolve_bare_function_key(key: KeyEvent) -> Action {
    keymap! { key;
        KeyCode::F(1) => Action::ToggleHelp,
        KeyCode::F(2) => Action::UserMenu,
        KeyCode::F(3) => Action::View,
        KeyCode::F(4) => Action::Edit,
        KeyCode::F(5) => Action::Copy,
        KeyCode::F(6) => Action::Move,
        KeyCode::F(7) => Action::Mkdir,
        KeyCode::F(8) => Action::DeleteConfirm,
        KeyCode::F(9) => Action::MenuBar,
        KeyCode::F(11) => Action::PluginCommands,
        // Far's F12 lists the open screens; Ctrl+O keeps the console toggle.
        KeyCode::F(12) => Action::Tool(ToolCommand::Screens),

        KeyCode::F(10) => Action::ConfirmQuit,
        _ => Action::QuickSearchClear,
    }
}

const fn resolve_alt_function_key(key: KeyEvent) -> Action {
    keymap! { key;
        KeyCode::F(1) => Action::DriveMenu(PanelSide::Left),
        KeyCode::F(2) => Action::DriveMenu(PanelSide::Right),
        KeyCode::F(4) => Action::EditExternal,
        KeyCode::F(6) => Action::File(FileCommand::Link),
        KeyCode::F(7) => Action::Tool(ToolCommand::FindFile),
        KeyCode::F(8) => Action::Tool(ToolCommand::History),
        KeyCode::F(9) => Action::Tool(ToolCommand::VideoMode),
        KeyCode::F(10) => Action::Tool(ToolCommand::FindFolder),
        KeyCode::F(11) => Action::Tool(ToolCommand::ViewHistory),
        KeyCode::F(12) => Action::Tool(ToolCommand::FolderHistory),
        _ => resolve_bare_function_key(key),
    }
}

const fn resolve_shift_function_key(key: KeyEvent) -> Action {
    keymap! { key;
        KeyCode::F(1) => Action::File(FileCommand::AddToArchive),
        KeyCode::F(2) => Action::File(FileCommand::Extract),
        KeyCode::F(3) => Action::File(FileCommand::ArchiveCommands),
        KeyCode::F(4) => Action::NewFile,
        KeyCode::F(6) => Action::Rename,
        KeyCode::F(9) => Action::Tool(ToolCommand::SaveSetup),
        _ => resolve_bare_function_key(key),
    }
}

/// `Ctrl+F12`, the sort modes menu, is a `MenuCommand` and resolved before
/// this table is reached.
const fn resolve_ctrl_function_key(key: KeyEvent) -> Action {
    keymap! { key;
        KeyCode::F(3) => Action::Panel(PanelCommand::Sort(SortMode::Name)),
        KeyCode::F(4) => Action::Panel(PanelCommand::Sort(SortMode::Extension)),
        KeyCode::F(5) => Action::Panel(PanelCommand::Sort(SortMode::ModificationTime)),
        KeyCode::F(6) => Action::Panel(PanelCommand::Sort(SortMode::Size)),
        KeyCode::F(7) => Action::Panel(PanelCommand::Sort(SortMode::Unsorted)),
        KeyCode::F(8) => Action::Panel(PanelCommand::Sort(SortMode::CreationTime)),
        KeyCode::F(9) => Action::Panel(PanelCommand::Sort(SortMode::AccessTime)),
        KeyCode::F(10) => Action::Panel(PanelCommand::Sort(SortMode::Descriptions)),
        KeyCode::F(11) => Action::Panel(PanelCommand::Sort(SortMode::Owner)),
        _ => resolve_bare_function_key(key),
    }
}
