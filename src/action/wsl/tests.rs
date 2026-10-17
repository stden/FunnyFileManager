//! The WSL dialog has separate bindings for platform and package updates.

use super::*;

#[test]
fn navigation_and_operations_resolve_to_distinct_commands() {
    for (key, command) in [
        (KeyCode::Up, WslCommand::Up),
        (KeyCode::Down, WslCommand::Down),
        (KeyCode::Home, WslCommand::First),
        (KeyCode::End, WslCommand::Last),
        (KeyCode::Enter, WslCommand::Open),
        (KeyCode::F(2), WslCommand::Refresh),
        (KeyCode::F(5), WslCommand::Update),
        (KeyCode::F(6), WslCommand::UpdatePackages),
        (KeyCode::Esc, WslCommand::Close),
    ] {
        assert_eq!(resolve(KeyEvent::new(key, KeyModifiers::NONE)), command);
    }
}

#[test]
fn unsupported_keys_and_modified_update_keys_are_consumed_without_operations() {
    for key in [KeyCode::F(8), KeyCode::Char('x'), KeyCode::Insert] {
        assert_eq!(
            resolve(KeyEvent::new(key, KeyModifiers::NONE)),
            WslCommand::Ignore
        );
    }
    for key in [KeyCode::F(2), KeyCode::F(5), KeyCode::F(6)] {
        for modifiers in [
            KeyModifiers::CONTROL,
            KeyModifiers::ALT,
            KeyModifiers::SHIFT,
        ] {
            assert_eq!(resolve(KeyEvent::new(key, modifiers)), WslCommand::Ignore);
        }
    }
}
