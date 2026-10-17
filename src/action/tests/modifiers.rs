//! Purpose: Modifier handling of the panels keymap. A chord is exactly the
//! keys it names: a bound chord answers, and a modifier the binding does not
//! have is another key, which the panels' fallback answers. The Ctrl letter
//! combos keep their panel meanings.
use super::*;

#[test]
fn test_resolve_modifier_qualified_precede_bare() {
    // Alt+F1/F2 are drive menus, not help; F2 bare is the user menu.
    assert_eq!(
        resolve(key_mod(KeyCode::F(1), KeyModifiers::ALT)),
        Action::DriveMenu(PanelSide::Left)
    );
    assert_eq!(
        resolve(key_mod(KeyCode::F(2), KeyModifiers::ALT)),
        Action::DriveMenu(PanelSide::Right)
    );
    assert_eq!(
        resolve(key_mod(KeyCode::F(2), KeyModifiers::empty())),
        Action::UserMenu
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
fn test_resolve_extra_modifiers_are_other_keys() {
    let cases = [
        (
            KeyCode::F(4),
            KeyModifiers::ALT | KeyModifiers::SHIFT,
            Action::QuickSearchClear,
        ),
        (
            KeyCode::F(1),
            KeyModifiers::ALT | KeyModifiers::CONTROL,
            Action::QuickSearchClear,
        ),
        (
            KeyCode::Char('u'),
            KeyModifiers::CONTROL | KeyModifiers::ALT,
            Action::QuickSearchClear,
        ),
        (
            KeyCode::Char('c'),
            KeyModifiers::CONTROL | KeyModifiers::ALT,
            Action::QuickSearchClear,
        ),
    ];
    assert_keymap(cases);
}

/// A chord nothing binds falls back: Alt with a character searches for it,
/// any other character opens the command line, and every other key clears the
/// quick search. Alt+F5 is not Copy, and Alt+Enter is not Open.
#[test]
fn test_resolve_unbound_modifier_chords_fall_back() {
    let cases = [
        (KeyCode::F(5), KeyModifiers::ALT, Action::QuickSearchClear),
        (KeyCode::Enter, KeyModifiers::ALT, Action::QuickSearchClear),
        (
            KeyCode::Char(' '),
            KeyModifiers::ALT,
            Action::QuickSearch(' '),
        ),
        (
            KeyCode::Char(':'),
            KeyModifiers::ALT,
            Action::QuickSearch(':'),
        ),
        (
            KeyCode::Backspace,
            KeyModifiers::SHIFT,
            Action::QuickSearchClear,
        ),
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
