//! Purpose: the panels' bound rows of the modified keys, and the keys nothing
//! binds. Ctrl letters, Alt+Delete and the Alt and Shift function keys name
//! the F9 menu's commands; a key nothing binds clears the quick search.
use super::*;

#[test]
fn bound_control_letters_and_alt_delete_name_panel_commands() {
    let ctrl = KeyModifiers::CONTROL;
    assert_eq!(
        resolve(key_mod(KeyCode::Char('q'), ctrl)),
        Action::Panel(PanelCommand::QuickView)
    );
    assert_eq!(
        resolve(key_mod(KeyCode::Char('n'), ctrl)),
        Action::Panel(PanelCommand::LongNames)
    );
    assert_eq!(
        resolve(key_mod(KeyCode::Delete, KeyModifiers::ALT)),
        Action::File(FileCommand::Wipe)
    );
}

#[test]
fn bound_control_letters_name_file_and_tool_commands() {
    let ctrl = KeyModifiers::CONTROL;
    assert_eq!(
        resolve(key_mod(KeyCode::Char('a'), ctrl)),
        Action::File(FileCommand::Attributes)
    );
    assert_eq!(
        resolve(key_mod(KeyCode::Char('g'), ctrl)),
        Action::File(FileCommand::ApplyCommand)
    );
    assert_eq!(
        resolve(key_mod(KeyCode::Char('z'), ctrl)),
        Action::File(FileCommand::Describe)
    );
    assert_eq!(
        resolve(key_mod(KeyCode::Char('i'), ctrl)),
        Action::Tool(ToolCommand::Filter)
    );
    assert_eq!(
        resolve(key_mod(KeyCode::Char('w'), ctrl)),
        Action::Tool(ToolCommand::Tasks)
    );
}

#[test]
fn bound_alt_function_keys_name_the_command_rows() {
    let alt = KeyModifiers::ALT;
    assert_eq!(
        resolve(key_mod(KeyCode::F(6), alt)),
        Action::File(FileCommand::Link)
    );
    assert_eq!(
        resolve(key_mod(KeyCode::F(7), alt)),
        Action::Tool(ToolCommand::FindFile)
    );
    assert_eq!(
        resolve(key_mod(KeyCode::F(8), alt)),
        Action::Tool(ToolCommand::History)
    );
    assert_eq!(
        resolve(key_mod(KeyCode::F(9), alt)),
        Action::Tool(ToolCommand::VideoMode)
    );
    assert_eq!(
        resolve(key_mod(KeyCode::F(10), alt)),
        Action::Tool(ToolCommand::FindFolder)
    );
    assert_eq!(
        resolve(key_mod(KeyCode::F(11), alt)),
        Action::Tool(ToolCommand::ViewHistory)
    );
    assert_eq!(
        resolve(key_mod(KeyCode::F(12), alt)),
        Action::Tool(ToolCommand::FolderHistory)
    );
}

#[test]
fn bound_shift_function_keys_name_the_file_rows() {
    let shift = KeyModifiers::SHIFT;
    assert_eq!(
        resolve(key_mod(KeyCode::F(1), shift)),
        Action::File(FileCommand::AddToArchive)
    );
    assert_eq!(
        resolve(key_mod(KeyCode::F(2), shift)),
        Action::File(FileCommand::Extract)
    );
    assert_eq!(
        resolve(key_mod(KeyCode::F(3), shift)),
        Action::File(FileCommand::ArchiveCommands)
    );
    // Shift+F9 is Save setup, the menu's own row, not a fall-through to F9.
    assert_eq!(
        resolve(key_mod(KeyCode::F(9), shift)),
        Action::Menu(MenuCommand::SaveSetup)
    );
}

#[test]
fn bound_ctrl_function_keys_toggle_the_panels() {
    // Ctrl+F1 is the left panel, not the bare F1 help.
    assert_eq!(
        resolve(key_mod(KeyCode::F(1), KeyModifiers::CONTROL)),
        Action::Panel(PanelCommand::TogglePanel(PanelSide::Left))
    );
}

#[test]
fn unbound_function_keys_and_control_characters_clear_quick_search() {
    assert_eq!(resolve(key(KeyCode::F(13))), Action::QuickSearchClear);
    assert_eq!(
        resolve(key_mod(KeyCode::Char('x'), KeyModifiers::CONTROL)),
        Action::QuickSearchClear
    );
}
