//! Contract tests for binding precedence and modifier policies.

use super::*;

#[test]
fn clipboard_bindings_require_the_exact_modifier_combination() {
    for (modifiers, command) in [
        (KeyModifiers::CONTROL, EditorCommand::Copy),
        (KeyModifiers::SHIFT, EditorCommand::Paste),
        (KeyModifiers::NONE, EditorCommand::Ignore),
        (KeyModifiers::ALT, EditorCommand::Ignore),
        (
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            EditorCommand::Ignore,
        ),
        (
            KeyModifiers::ALT | KeyModifiers::SHIFT,
            EditorCommand::Ignore,
        ),
    ] {
        assert_eq!(resolve(KeyEvent::new(KeyCode::Insert, modifiers)), command);
    }
}

#[test]
fn ctrl_c_copies_for_both_character_cases_and_rejects_extra_modifiers() {
    for ch in ['c', 'C'] {
        assert_eq!(
            resolve(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::CONTROL)),
            EditorCommand::Copy
        );
        for extra in [KeyModifiers::SHIFT, KeyModifiers::ALT, KeyModifiers::SUPER] {
            assert_eq!(
                resolve(KeyEvent::new(
                    KeyCode::Char(ch),
                    KeyModifiers::CONTROL | extra
                )),
                EditorCommand::Ignore
            );
        }
    }
}

#[test]
fn ctrl_v_pastes_and_modified_variants_remain_unsupported() {
    for ch in ['v', 'V'] {
        assert_eq!(
            resolve(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::CONTROL)),
            EditorCommand::Paste
        );
        for extra in [KeyModifiers::SHIFT, KeyModifiers::ALT] {
            assert_eq!(
                resolve(KeyEvent::new(
                    KeyCode::Char(ch),
                    KeyModifiers::CONTROL | extra
                )),
                EditorCommand::Ignore
            );
        }
    }
}

#[test]
fn navigation_carries_selection_intent_without_changing_an_editor() {
    for (code, motion) in [
        (KeyCode::Left, EditorNavigation::Left),
        (KeyCode::Right, EditorNavigation::Right),
        (KeyCode::Up, EditorNavigation::Up),
        (KeyCode::Down, EditorNavigation::Down),
        (KeyCode::Home, EditorNavigation::Home),
        (KeyCode::End, EditorNavigation::End),
        (KeyCode::PageUp, EditorNavigation::PageUp),
        (KeyCode::PageDown, EditorNavigation::PageDown),
    ] {
        for (modifiers, selection) in [
            (KeyModifiers::NONE, SelectionMode::Move),
            (KeyModifiers::SHIFT, SelectionMode::Extend),
            (
                KeyModifiers::CONTROL | KeyModifiers::SHIFT,
                SelectionMode::Extend,
            ),
        ] {
            assert_eq!(
                resolve(KeyEvent::new(code, modifiers)),
                EditorCommand::Move { motion, selection }
            );
        }
    }
}

#[test]
fn qualified_search_wins_and_plain_commands_keep_their_modifier_policy() {
    for (code, modifiers, command) in [
        (
            KeyCode::F(7),
            KeyModifiers::SHIFT | KeyModifiers::ALT,
            EditorCommand::RepeatSearch,
        ),
        (KeyCode::F(7), KeyModifiers::NONE, EditorCommand::Search),
        (KeyCode::F(2), KeyModifiers::CONTROL, EditorCommand::Save),
        (KeyCode::Esc, KeyModifiers::NONE, EditorCommand::Close),
        (KeyCode::F(10), KeyModifiers::NONE, EditorCommand::Close),
        (KeyCode::Enter, KeyModifiers::NONE, EditorCommand::Newline),
        (
            KeyCode::Backspace,
            KeyModifiers::NONE,
            EditorCommand::Backspace,
        ),
        (KeyCode::Delete, KeyModifiers::NONE, EditorCommand::Delete),
        (KeyCode::Tab, KeyModifiers::SHIFT, EditorCommand::Indent),
    ] {
        assert_eq!(resolve(KeyEvent::new(code, modifiers)), command);
    }
}

#[test]
fn text_input_accepts_unicode_and_consumes_control_sequences() {
    for (code, modifiers, command) in [
        (
            KeyCode::Char('界'),
            KeyModifiers::NONE,
            EditorCommand::Insert('界'),
        ),
        (
            KeyCode::Char('Ж'),
            KeyModifiers::SHIFT,
            EditorCommand::Insert('Ж'),
        ),
        (
            KeyCode::Char('x'),
            KeyModifiers::CONTROL,
            EditorCommand::Ignore,
        ),
        (KeyCode::Char('x'), KeyModifiers::ALT, EditorCommand::Ignore),
        (KeyCode::Null, KeyModifiers::NONE, EditorCommand::Ignore),
    ] {
        assert_eq!(resolve(KeyEvent::new(code, modifiers)), command);
    }
}
#[test]
fn lsp_bindings_are_explicit_and_require_exact_modifiers() {
    use crate::models::LspFeature;
    for (key, feature) in [
        (' ', LspFeature::Completion),
        ('k', LspFeature::Hover),
        (']', LspFeature::Definition),
    ] {
        assert_eq!(
            resolve(KeyEvent::new(KeyCode::Char(key), KeyModifiers::CONTROL)),
            EditorCommand::Lsp(feature)
        );
        assert_eq!(
            resolve(KeyEvent::new(
                KeyCode::Char(key),
                KeyModifiers::CONTROL | KeyModifiers::ALT
            )),
            EditorCommand::Ignore
        );
    }
    assert_eq!(
        resolve(KeyEvent::new(
            KeyCode::Char('r'),
            KeyModifiers::CONTROL | KeyModifiers::ALT
        )),
        EditorCommand::RestartLsp
    );
}

/// A terminal without the kitty protocol sends Ctrl+] as byte 0x1D, which
/// crossterm reports as Ctrl+5. The editor has no Ctrl+5 of its own, so that
/// is go to definition too.
#[test]
fn ctrl_5_is_a_legacy_terminals_ctrl_bracket_and_goes_to_definition() {
    assert_eq!(
        resolve(KeyEvent::new(KeyCode::Char('5'), KeyModifiers::CONTROL)),
        EditorCommand::Lsp(crate::models::LspFeature::Definition)
    );
}

#[test]
fn ctrl_alt_l_formats_document_for_both_cases_and_rejects_extra_modifiers() {
    let ctrl_alt = KeyModifiers::CONTROL | KeyModifiers::ALT;
    for ch in ['l', 'L'] {
        assert_eq!(
            resolve(KeyEvent::new(KeyCode::Char(ch), ctrl_alt)),
            EditorCommand::FormatDocument
        );
        assert_eq!(
            resolve(KeyEvent::new(
                KeyCode::Char(ch),
                ctrl_alt | KeyModifiers::SHIFT
            )),
            EditorCommand::Ignore
        );
    }
}
