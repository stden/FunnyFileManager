//! Additional main-menu shortcuts, resolved before legacy fallback bindings.

use crate::{menu::MenuCommand as Command, models::PanelSide};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub(super) const fn resolve(key: KeyEvent) -> Option<Command> {
    keymap! { key;
        KeyCode::Char('t') if exact(CONTROL) => Some(Command::Tree),
        KeyCode::Char('q') if exact(CONTROL) => Some(Command::QuickView),
        KeyCode::Char('n') if exact(CONTROL) => Some(Command::LongNames),
        KeyCode::Char('a') if exact(CONTROL) => Some(Command::Attributes),
        KeyCode::Char('g') if exact(CONTROL) => Some(Command::ApplyCommand),
        KeyCode::Char('z') if exact(CONTROL) => Some(Command::Describe),
        KeyCode::Char('i') if exact(CONTROL) => Some(Command::Filter),
        KeyCode::Char('w') if exact(CONTROL) => Some(Command::Tasks),
        KeyCode::Delete if exact(ALT) => Some(Command::Wipe),
        _ => function_key(key),
    }
}

const fn function_key(key: KeyEvent) -> Option<Command> {
    keymap! { key;
        KeyCode::F(1) if exact(CONTROL) => Some(Command::TogglePanel(PanelSide::Left)),
        KeyCode::F(2) if exact(CONTROL) => Some(Command::TogglePanel(PanelSide::Right)),
        KeyCode::F(12) if exact(CONTROL) => Some(Command::Sort),
        KeyCode::F(1) if exact(SHIFT) => Some(Command::Archive),
        KeyCode::F(2) if exact(SHIFT) => Some(Command::Extract),
        KeyCode::F(3) if exact(SHIFT) => Some(Command::ArchiveCommands),
        KeyCode::F(9) if exact(SHIFT) => Some(Command::SaveSetup),
        KeyCode::F(6) if exact(ALT) => Some(Command::Link),
        KeyCode::F(7) if exact(ALT) => Some(Command::FindFile),
        KeyCode::F(8) if exact(ALT) => Some(Command::History),
        KeyCode::F(9) if exact(ALT) => Some(Command::VideoMode),
        KeyCode::F(10) if exact(ALT) => Some(Command::FindFolder),
        KeyCode::F(11) if exact(ALT) => Some(Command::ViewHistory),
        KeyCode::F(12) if exact(ALT) => Some(Command::FolderHistory),
        KeyCode::F(12) if exact(NONE) => Some(Command::Screens),
        _ => None,
    }
}
