//! Purpose: Modifier handling of the main-view keymap — qualified keys precede
//! bare ones, extra modifiers are contained, unsupported ones fall back to the
//! bare key, and the Ctrl letter combos keep their panel meanings.
use super::*;

#[test]
fn test_resolve_modifier_qualified_precede_bare() {
    // Alt+F1/F2 are drive menus, not help.
    assert_eq!(
        resolve(key_mod(KeyCode::F(1), KeyModifiers::ALT)),
        Action::DriveMenu(PanelSide::Left)
    );
    assert_eq!(
        resolve(key_mod(KeyCode::F(2), KeyModifiers::ALT)),
        Action::DriveMenu(PanelSide::Right)
    );
    // Shift+F4 is "new file", Alt+F4 is "external editor" — not bare "edit".
    assert_eq!(
        resolve(key_mod(KeyCode::F(4), KeyModifiers::SHIFT)),
        Action::NewFile
    );
    assert_eq!(
        resolve(key_mod(KeyCode::F(4), KeyModifiers::ALT)),
        Action::EditExternal
    );
}

#[test]
fn test_resolve_dsl_preserves_order_and_modifier_containment() {
    let cases = [
        (
            KeyCode::F(4),
            KeyModifiers::ALT | KeyModifiers::SHIFT,
            Action::EditExternal,
        ),
        (
            KeyCode::F(1),
            KeyModifiers::ALT | KeyModifiers::CONTROL,
            Action::DriveMenu(PanelSide::Left),
        ),
        (
            KeyCode::Char('u'),
            KeyModifiers::CONTROL | KeyModifiers::ALT,
            Action::SwapPanels,
        ),
        (
            KeyCode::Char('c'),
            KeyModifiers::CONTROL | KeyModifiers::ALT,
            Action::Quit,
        ),
    ];
    assert_keymap(cases);
}

#[test]
fn test_resolve_unsupported_modifiers_keep_key_behavior() {
    let cases = [
        (KeyCode::F(5), KeyModifiers::ALT, Action::Copy),
        (KeyCode::Enter, KeyModifiers::ALT, Action::Open),
        (KeyCode::Char(' '), KeyModifiers::ALT, Action::ToggleMark),
        (KeyCode::Char(':'), KeyModifiers::ALT, Action::CommandLine),
        (
            KeyCode::Backspace,
            KeyModifiers::SHIFT,
            Action::QuickSearchBackspace,
        ),
        (KeyCode::F(2), KeyModifiers::empty(), Action::UserMenu),
        (
            KeyCode::Char('C'),
            KeyModifiers::CONTROL,
            Action::QuickSearchClear,
        ),
    ];

    assert_keymap(cases);
}

#[test]
fn test_resolve_ctrl_combos() {
    assert_eq!(
        resolve(key_mod(KeyCode::Char('\\'), KeyModifiers::CONTROL)),
        Action::GoRoot
    );
    assert_eq!(
        resolve(key_mod(KeyCode::Char('u'), KeyModifiers::CONTROL)),
        Action::SwapPanels
    );
    assert_eq!(
        resolve(key_mod(KeyCode::Char('o'), KeyModifiers::CONTROL)),
        Action::ToggleConsole
    );
    assert_eq!(
        resolve(key_mod(KeyCode::Char('r'), KeyModifiers::CONTROL)),
        Action::Refresh
    );
}

fn assert_keymap(cases: impl IntoIterator<Item = (KeyCode, KeyModifiers, Action)>) {
    for (code, modifiers, expected) in cases {
        assert_eq!(resolve(key_mod(code, modifiers)), expected);
    }
}
