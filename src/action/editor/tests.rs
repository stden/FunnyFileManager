//! Contract tests for the editor's keys: the keymap's defaults, matched
//! with exactly their modifiers, and what a key means that nothing is bound to.

mod selection;

use super::*;
use crate::keymap::Keymap;
use crate::models::LspFeature;

mod fallback;
mod letters;
mod remapped;

const NONE: KeyModifiers = KeyModifiers::NONE;
const SHIFT: KeyModifiers = KeyModifiers::SHIFT;
const ALT: KeyModifiers = KeyModifiers::ALT;
const CTRL: KeyModifiers = KeyModifiers::CONTROL;

/// The command the default keys give the key.
fn command(code: KeyCode, modifiers: KeyModifiers) -> EditorCommand {
    resolve(&Keymap::default().editor, KeyEvent::new(code, modifiers))
}

const fn moves(motion: EditorNavigation, selection: SelectionMode) -> EditorCommand {
    EditorCommand::Move { motion, selection }
}

/// Each key that runs its command with the modifiers shown: the plain keys,
/// Shift+F7 and Ctrl+Insert.
const PLAIN_KEYS: [(KeyCode, KeyModifiers, EditorCommand); 11] = [
    (KeyCode::F(2), NONE, EditorCommand::Save),
    (KeyCode::F(7), NONE, EditorCommand::Search),
    (KeyCode::F(7), SHIFT, EditorCommand::RepeatSearch),
    (KeyCode::F(8), NONE, EditorCommand::ChooseEncoding),
    (KeyCode::F(10), NONE, EditorCommand::Close),
    (KeyCode::Esc, NONE, EditorCommand::Close),
    (KeyCode::Enter, NONE, EditorCommand::Newline),
    (KeyCode::Backspace, NONE, EditorCommand::Backspace),
    (KeyCode::Delete, NONE, EditorCommand::Delete),
    (KeyCode::Tab, NONE, EditorCommand::Indent),
    (KeyCode::Insert, CTRL, EditorCommand::Copy),
];

/// Keys the editor obeyed while it ignored the modifiers nothing was bound
/// with. Exact matching makes each one fall to the fallback, which runs
/// nothing: a sample of the 106 rows `tests/keymap_golden/editor.tsv` lists
/// as `fallback`.
const ONCE_LENIENT: [(KeyCode, KeyModifiers); 14] = [
    (KeyCode::F(2), SHIFT),
    (KeyCode::F(2), CTRL),
    (KeyCode::F(7), ALT),
    (KeyCode::F(7), SHIFT.union(ALT)),
    (KeyCode::F(8), SHIFT),
    (KeyCode::F(10), ALT),
    (KeyCode::Esc, SHIFT),
    (KeyCode::Enter, SHIFT),
    (KeyCode::Enter, CTRL),
    (KeyCode::Backspace, ALT),
    (KeyCode::Delete, SHIFT),
    (KeyCode::Tab, CTRL),
    (KeyCode::Tab, SHIFT),
    (KeyCode::BackTab, SHIFT),
];

const MOTIONS: [(KeyCode, EditorNavigation); 8] = [
    (KeyCode::Left, EditorNavigation::Left),
    (KeyCode::Right, EditorNavigation::Right),
    (KeyCode::Up, EditorNavigation::Up),
    (KeyCode::Down, EditorNavigation::Down),
    (KeyCode::Home, EditorNavigation::Home),
    (KeyCode::End, EditorNavigation::End),
    (KeyCode::PageUp, EditorNavigation::PageUp),
    (KeyCode::PageDown, EditorNavigation::PageDown),
];

#[test]
fn the_default_keys_run_their_commands() {
    for (code, modifiers, expected) in PLAIN_KEYS {
        assert_eq!(command(code, modifiers), expected, "{modifiers:?} {code:?}");
    }
}

#[test]
fn a_modifier_nothing_is_bound_with_makes_another_key_which_runs_nothing() {
    for (code, modifiers) in ONCE_LENIENT {
        assert_eq!(
            command(code, modifiers),
            EditorCommand::Ignore,
            "{modifiers:?} {code:?}"
        );
    }
}

#[test]
fn the_caret_moves_bare_and_selects_with_shift_alone() {
    for (code, motion) in MOTIONS {
        for (modifiers, expected) in [
            (NONE, moves(motion, SelectionMode::Move)),
            (SHIFT, moves(motion, SelectionMode::Extend)),
            (CTRL, EditorCommand::Ignore),
            (ALT, EditorCommand::Ignore),
            (CTRL.union(SHIFT), EditorCommand::Ignore),
        ] {
            assert_eq!(command(code, modifiers), expected, "{modifiers:?} {code:?}");
        }
    }
}

#[test]
fn insert_copies_with_ctrl_and_pastes_with_shift_and_nothing_else() {
    for (modifiers, expected) in [
        (CTRL, EditorCommand::Copy),
        (SHIFT, EditorCommand::Paste),
        (NONE, EditorCommand::Ignore),
        (ALT, EditorCommand::Ignore),
        (CTRL.union(SHIFT), EditorCommand::Ignore),
        (ALT.union(SHIFT), EditorCommand::Ignore),
    ] {
        assert_eq!(command(KeyCode::Insert, modifiers), expected);
    }
}

/// A desktop that passes Super, Hyper or Meta through makes no key dead.
#[test]
fn super_hyper_and_meta_are_not_part_of_a_key() {
    for extra in [KeyModifiers::SUPER, KeyModifiers::HYPER, KeyModifiers::META] {
        assert_eq!(
            command(KeyCode::Char('c'), CTRL.union(extra)),
            EditorCommand::Copy
        );
        assert_eq!(command(KeyCode::F(2), extra), EditorCommand::Save);
        assert_eq!(
            command(KeyCode::Char('x'), extra),
            EditorCommand::Insert('x')
        );
    }
}

#[test]
fn the_language_servers_keys_require_exact_modifiers() {
    for (key, feature) in [
        (' ', LspFeature::Completion),
        ('k', LspFeature::Hover),
        (']', LspFeature::Definition),
    ] {
        assert_eq!(
            command(KeyCode::Char(key), CTRL),
            EditorCommand::Lsp(feature)
        );
        assert_eq!(
            command(KeyCode::Char(key), CTRL.union(ALT)),
            EditorCommand::Ignore
        );
    }
    assert_eq!(
        command(KeyCode::Char('r'), CTRL.union(ALT)),
        EditorCommand::RestartLsp
    );
    assert_eq!(command(KeyCode::Char('r'), CTRL), EditorCommand::Ignore);
}

/// A terminal without the kitty protocol sends Ctrl+] as byte 0x1D, which
/// crossterm reports as Ctrl+5. The editor has no Ctrl+5 of its own, so that
/// is go to definition too.
#[test]
fn ctrl_5_is_a_legacy_terminals_ctrl_bracket_and_goes_to_definition() {
    assert_eq!(
        command(KeyCode::Char('5'), CTRL),
        EditorCommand::Lsp(LspFeature::Definition)
    );
}
