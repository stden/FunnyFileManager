//! Purpose: The `resolve_main` fallback rows — Ctrl letters and Alt/Shift
//! function keys still name the F9 menu commands, and unused keys clear the
//! quick search.
use super::*;

#[test]
fn fallback_control_letters_still_name_panel_commands() {
    let ctrl = KeyModifiers::CONTROL;
    assert_eq!(
        resolve_main(key_mod(KeyCode::Char('q'), ctrl)),
        Action::Panel(PanelCommand::QuickView)
    );
    assert_eq!(
        resolve_main(key_mod(KeyCode::Char('n'), ctrl)),
        Action::Panel(PanelCommand::LongNames)
    );
    assert_eq!(
        resolve_main(key_mod(KeyCode::Delete, KeyModifiers::ALT)),
        Action::File(FileCommand::Wipe)
    );
}

#[test]
fn fallback_control_letters_still_name_file_and_tool_commands() {
    let ctrl = KeyModifiers::CONTROL;
    assert_eq!(
        resolve_main(key_mod(KeyCode::Char('a'), ctrl)),
        Action::File(FileCommand::Attributes)
    );
    assert_eq!(
        resolve_main(key_mod(KeyCode::Char('g'), ctrl)),
        Action::File(FileCommand::ApplyCommand)
    );
    assert_eq!(
        resolve_main(key_mod(KeyCode::Char('z'), ctrl)),
        Action::File(FileCommand::Describe)
    );
    assert_eq!(
        resolve_main(key_mod(KeyCode::Char('i'), ctrl)),
        Action::Tool(ToolCommand::Filter)
    );
    assert_eq!(
        resolve_main(key_mod(KeyCode::Char('w'), ctrl)),
        Action::Tool(ToolCommand::Tasks)
    );
}

#[test]
fn fallback_alt_function_keys_still_name_the_command_rows() {
    let alt = KeyModifiers::ALT;
    assert_eq!(
        resolve_main(key_mod(KeyCode::F(6), alt)),
        Action::File(FileCommand::Link)
    );
    assert_eq!(
        resolve_main(key_mod(KeyCode::F(7), alt)),
        Action::Tool(ToolCommand::FindFile)
    );
    assert_eq!(
        resolve_main(key_mod(KeyCode::F(8), alt)),
        Action::Tool(ToolCommand::History)
    );
    assert_eq!(
        resolve_main(key_mod(KeyCode::F(9), alt)),
        Action::Tool(ToolCommand::VideoMode)
    );
    assert_eq!(
        resolve_main(key_mod(KeyCode::F(10), alt)),
        Action::Tool(ToolCommand::FindFolder)
    );
    assert_eq!(
        resolve_main(key_mod(KeyCode::F(11), alt)),
        Action::Tool(ToolCommand::ViewHistory)
    );
    assert_eq!(
        resolve_main(key_mod(KeyCode::F(12), alt)),
        Action::Tool(ToolCommand::FolderHistory)
    );
}

#[test]
fn fallback_shift_function_keys_still_name_the_file_rows() {
    let shift = KeyModifiers::SHIFT;
    assert_eq!(
        resolve_main(key_mod(KeyCode::F(1), shift)),
        Action::File(FileCommand::AddToArchive)
    );
    assert_eq!(
        resolve_main(key_mod(KeyCode::F(2), shift)),
        Action::File(FileCommand::Extract)
    );
    assert_eq!(
        resolve_main(key_mod(KeyCode::F(3), shift)),
        Action::File(FileCommand::ArchiveCommands)
    );
    assert_eq!(
        resolve_main(key_mod(KeyCode::F(9), shift)),
        Action::Tool(ToolCommand::SaveSetup)
    );
}

#[test]
fn unused_function_keys_and_control_characters_clear_quick_search() {
    assert_eq!(resolve_main(key(KeyCode::F(13))), Action::QuickSearchClear);
    assert_eq!(
        resolve_character('x', KeyModifiers::CONTROL),
        Action::QuickSearchClear
    );
    assert_eq!(
        resolve_main(key_mod(KeyCode::F(1), KeyModifiers::CONTROL)),
        Action::ToggleHelp
    );
}
