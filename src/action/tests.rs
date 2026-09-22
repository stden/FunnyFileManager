use super::*;

mod fallback_rows;
mod modifiers;
mod quick_search;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}
fn key_mod(code: KeyCode, m: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, m)
}

#[test]
fn test_resolve_quit_variants() {
    // F10 asks first, as Far does.
    assert_eq!(resolve(key(KeyCode::F(10))), Action::ConfirmQuit);
    // Ctrl+C does not: the emergency exit stays an emergency exit, and a
    // dialog in front of it is one keystroke between the user and a
    // program they have decided to stop.
    assert_eq!(
        resolve(key_mod(KeyCode::Char('c'), KeyModifiers::CONTROL)),
        Action::Quit
    );
}

/// Bare `q` starts the command `quit` instead of quitting outright — a
/// command line you cannot type `q` into cannot run `quit` or `qemu`.
#[test]
fn test_resolve_bare_q_types_instead_of_quitting() {
    assert_eq!(
        resolve(key(KeyCode::Char('q'))),
        Action::CommandLineChar('q')
    );
    assert_eq!(
        resolve(key(KeyCode::Char('Q'))),
        Action::CommandLineChar('Q')
    );
}

/// Every printable key opens the prompt, except the ones the main view
/// still claims: `Space` marks, `:` opens an empty prompt, Ctrl combos and
/// Alt+character keep their own meanings.
#[test]
fn test_resolve_printable_keys_start_a_command() {
    assert_eq!(
        resolve(key(KeyCode::Char('l'))),
        Action::CommandLineChar('l')
    );
    assert_eq!(
        resolve(key(KeyCode::Char('/'))),
        Action::CommandLineChar('/')
    );
    assert_eq!(resolve(key(KeyCode::Char(' '))), Action::ToggleMark);
    assert_eq!(resolve(key(KeyCode::Char(':'))), Action::CommandLine);
    assert_eq!(
        resolve(key_mod(KeyCode::Char('u'), KeyModifiers::CONTROL)),
        Action::SwapPanels
    );
    assert_eq!(
        resolve(key_mod(KeyCode::Char('a'), KeyModifiers::ALT)),
        Action::QuickSearch('a')
    );
}

#[test]
fn test_resolve_navigation() {
    assert_eq!(resolve(key(KeyCode::Down)), Action::NavDown);
    assert_eq!(resolve(key(KeyCode::Up)), Action::NavUp);
    assert_eq!(resolve(key(KeyCode::Enter)), Action::Open);
    assert_eq!(resolve(key(KeyCode::Tab)), Action::SwitchPanel);
    assert_eq!(
        resolve(key(KeyCode::Left)),
        Action::SelectPanel(PanelSide::Left)
    );
    assert_eq!(
        resolve(key(KeyCode::Right)),
        Action::SelectPanel(PanelSide::Right)
    );
    assert_eq!(resolve(key(KeyCode::Home)), Action::First);
    assert_eq!(resolve(key(KeyCode::End)), Action::Last);
}

#[test]
fn test_resolve_paging() {
    assert_eq!(resolve(key(KeyCode::PageUp)), Action::PageUp);
    assert_eq!(resolve(key(KeyCode::PageDown)), Action::PageDown);
}

#[test]
fn test_resolve_fkeys() {
    assert_eq!(resolve(key(KeyCode::F(1))), Action::ToggleHelp);
    assert_eq!(resolve(key(KeyCode::F(2))), Action::UserMenu);
    assert_eq!(resolve(key(KeyCode::F(3))), Action::View);
    assert_eq!(resolve(key(KeyCode::F(4))), Action::Edit);
    assert_eq!(resolve(key(KeyCode::F(5))), Action::Copy);
    assert_eq!(resolve(key(KeyCode::F(6))), Action::Move);
    assert_eq!(resolve(key(KeyCode::F(7))), Action::Mkdir);
    assert_eq!(resolve(key(KeyCode::F(8))), Action::DeleteConfirm);
    assert_eq!(resolve(key(KeyCode::F(9))), Action::MenuBar);
    assert_eq!(resolve(key(KeyCode::F(11))), Action::McpStatus);
    // Far's F12 lists the screens; the console toggle stays on Ctrl+O.
    assert_eq!(
        resolve(key(KeyCode::F(12))),
        Action::MenuCommand(crate::menu::MenuCommand::Screens)
    );
}

#[test]
fn test_resolve_marking_and_command_line() {
    assert_eq!(resolve(key(KeyCode::Insert)), Action::ToggleMark);
    assert_eq!(resolve(key(KeyCode::Char(' '))), Action::ToggleMark);
    assert_eq!(resolve(key(KeyCode::Char(':'))), Action::CommandLine);
}
