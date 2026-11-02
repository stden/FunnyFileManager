//! Purpose: the Far Manager editor keys that FFM binds to no command, pinned one
//! key event each, so that a key bound later fails here and the list in the
//! knowledge base changes with it.

use crossterm::event::{KeyCode, KeyModifiers, ModifierKeyCode};

use super::{command, ALT, CTRL, NONE, SHIFT};
use crate::action::editor::EditorCommand;
use crate::models::LspFeature;

const CTRL_SHIFT: KeyModifiers = CTRL.union(SHIFT);
const ALT_SHIFT: KeyModifiers = ALT.union(SHIFT);
const CTRL_ALT: KeyModifiers = CTRL.union(ALT);
const CTRL_ALT_SHIFT: KeyModifiers = CTRL.union(ALT).union(SHIFT);

/// The Far editor keys of FarEng.hlf.m4 lines 3159 to 3241 that FFM binds to no
/// command, one row per key event, in the order of the help. "Cursor keys" is
/// read as the eight caret keys, the arrows and Home, End, `PgUp` and `PgDn`,
/// which is how the Shift+cursor selection reads them. Ctrl+K is the one row
/// that runs a command: Far's delete to the end of the line is missing, and
/// FFM's Ctrl+K is the language server's hover, which the row pins. Ctrl+5 is
/// FFM's Ctrl+] (go to definition), so the Ctrl+digit bookmark rows skip 5;
/// Ctrl+Shift+5 is unbound.
const UNIMPLEMENTED: [(KeyCode, KeyModifiers, EditorCommand); 94] = [
    // Cursor movement (3162 to 3177)
    (KeyCode::Char('s'), CTRL, EditorCommand::Ignore),
    (KeyCode::Left, CTRL, EditorCommand::Ignore),
    (KeyCode::Right, CTRL, EditorCommand::Ignore),
    (KeyCode::Up, CTRL, EditorCommand::Ignore),
    (KeyCode::Down, CTRL, EditorCommand::Ignore),
    (KeyCode::Char('n'), CTRL, EditorCommand::Ignore),
    (KeyCode::Char('e'), CTRL, EditorCommand::Ignore),
    // Delete operations (3183 to 3186)
    (KeyCode::Char('y'), CTRL, EditorCommand::Ignore),
    (
        KeyCode::Char('k'),
        CTRL,
        EditorCommand::Lsp(LspFeature::Hover),
    ),
    (KeyCode::Backspace, CTRL, EditorCommand::Ignore),
    (KeyCode::Char('t'), CTRL, EditorCommand::Ignore),
    (KeyCode::Delete, CTRL, EditorCommand::Ignore),
    // Block operations (3191 to 3205)
    (KeyCode::Left, CTRL_SHIFT, EditorCommand::Ignore),
    (KeyCode::Right, CTRL_SHIFT, EditorCommand::Ignore),
    (KeyCode::Up, CTRL_SHIFT, EditorCommand::Ignore),
    (KeyCode::Down, CTRL_SHIFT, EditorCommand::Ignore),
    (KeyCode::PageUp, CTRL_SHIFT, EditorCommand::Ignore),
    (KeyCode::PageDown, CTRL_SHIFT, EditorCommand::Ignore),
    (KeyCode::Left, ALT, EditorCommand::Ignore),
    (KeyCode::Right, ALT, EditorCommand::Ignore),
    (KeyCode::Up, ALT, EditorCommand::Ignore),
    (KeyCode::Down, ALT, EditorCommand::Ignore),
    (KeyCode::Home, ALT, EditorCommand::Ignore),
    (KeyCode::End, ALT, EditorCommand::Ignore),
    (KeyCode::PageUp, ALT, EditorCommand::Ignore),
    (KeyCode::PageDown, ALT, EditorCommand::Ignore),
    (KeyCode::Left, ALT_SHIFT, EditorCommand::Ignore),
    (KeyCode::Right, ALT_SHIFT, EditorCommand::Ignore),
    (KeyCode::Up, ALT_SHIFT, EditorCommand::Ignore),
    (KeyCode::Down, ALT_SHIFT, EditorCommand::Ignore),
    (KeyCode::Home, ALT_SHIFT, EditorCommand::Ignore),
    (KeyCode::End, ALT_SHIFT, EditorCommand::Ignore),
    (KeyCode::PageUp, ALT_SHIFT, EditorCommand::Ignore),
    (KeyCode::PageDown, ALT_SHIFT, EditorCommand::Ignore),
    (KeyCode::Left, CTRL_ALT, EditorCommand::Ignore),
    (KeyCode::Right, CTRL_ALT, EditorCommand::Ignore),
    (KeyCode::Up, CTRL_ALT, EditorCommand::Ignore),
    (KeyCode::Down, CTRL_ALT, EditorCommand::Ignore),
    (KeyCode::Home, CTRL_ALT, EditorCommand::Ignore),
    (KeyCode::End, CTRL_ALT, EditorCommand::Ignore),
    (KeyCode::PageUp, CTRL_ALT, EditorCommand::Ignore),
    (KeyCode::PageDown, CTRL_ALT, EditorCommand::Ignore),
    (KeyCode::Char('u'), CTRL, EditorCommand::Ignore),
    (KeyCode::Delete, SHIFT, EditorCommand::Ignore),
    (KeyCode::Char('x'), CTRL, EditorCommand::Ignore),
    (KeyCode::Char('+'), CTRL, EditorCommand::Ignore),
    (KeyCode::Char('d'), CTRL, EditorCommand::Ignore),
    (KeyCode::Char('p'), CTRL, EditorCommand::Ignore),
    (KeyCode::Char('m'), CTRL, EditorCommand::Ignore),
    (KeyCode::Char('u'), ALT, EditorCommand::Ignore),
    (KeyCode::Char('i'), ALT, EditorCommand::Ignore),
    // Other operations (3209 to 3241)
    (KeyCode::F(1), NONE, EditorCommand::Ignore),
    (KeyCode::F(2), SHIFT, EditorCommand::Ignore),
    (KeyCode::F(4), SHIFT, EditorCommand::Ignore),
    (KeyCode::F(6), NONE, EditorCommand::Ignore),
    (KeyCode::F(7), CTRL, EditorCommand::Ignore),
    (KeyCode::F(7), ALT, EditorCommand::Ignore),
    (KeyCode::F(8), ALT, EditorCommand::Ignore),
    (KeyCode::F(9), ALT, EditorCommand::Ignore),
    (KeyCode::F(9), ALT_SHIFT, EditorCommand::Ignore),
    (KeyCode::F(10), SHIFT, EditorCommand::Ignore),
    (KeyCode::F(10), CTRL, EditorCommand::Ignore),
    (KeyCode::F(11), NONE, EditorCommand::Ignore),
    (KeyCode::F(11), ALT, EditorCommand::Ignore),
    (KeyCode::Backspace, ALT, EditorCommand::Ignore),
    (KeyCode::Char('z'), CTRL, EditorCommand::Ignore),
    (KeyCode::Char('z'), CTRL_SHIFT, EditorCommand::Ignore),
    (KeyCode::Char('l'), CTRL, EditorCommand::Ignore),
    (KeyCode::Char('o'), CTRL, EditorCommand::Ignore),
    (
        KeyCode::Modifier(ModifierKeyCode::LeftShift),
        CTRL_ALT_SHIFT,
        EditorCommand::Ignore,
    ),
    (KeyCode::Char('q'), CTRL, EditorCommand::Ignore),
    // RightCtrl+0…9 sets a bookmark and LeftCtrl+0…9 goes to one; both arrive as Ctrl+digit.
    (KeyCode::Char('0'), CTRL, EditorCommand::Ignore),
    (KeyCode::Char('1'), CTRL, EditorCommand::Ignore),
    (KeyCode::Char('2'), CTRL, EditorCommand::Ignore),
    (KeyCode::Char('3'), CTRL, EditorCommand::Ignore),
    (KeyCode::Char('4'), CTRL, EditorCommand::Ignore),
    (KeyCode::Char('6'), CTRL, EditorCommand::Ignore),
    (KeyCode::Char('7'), CTRL, EditorCommand::Ignore),
    (KeyCode::Char('8'), CTRL, EditorCommand::Ignore),
    (KeyCode::Char('9'), CTRL, EditorCommand::Ignore),
    // Ctrl+Shift+0…9 sets a bookmark.
    (KeyCode::Char('0'), CTRL_SHIFT, EditorCommand::Ignore),
    (KeyCode::Char('1'), CTRL_SHIFT, EditorCommand::Ignore),
    (KeyCode::Char('2'), CTRL_SHIFT, EditorCommand::Ignore),
    (KeyCode::Char('3'), CTRL_SHIFT, EditorCommand::Ignore),
    (KeyCode::Char('4'), CTRL_SHIFT, EditorCommand::Ignore),
    (KeyCode::Char('5'), CTRL_SHIFT, EditorCommand::Ignore),
    (KeyCode::Char('6'), CTRL_SHIFT, EditorCommand::Ignore),
    (KeyCode::Char('7'), CTRL_SHIFT, EditorCommand::Ignore),
    (KeyCode::Char('8'), CTRL_SHIFT, EditorCommand::Ignore),
    (KeyCode::Char('9'), CTRL_SHIFT, EditorCommand::Ignore),
    (KeyCode::Enter, SHIFT, EditorCommand::Ignore),
    (KeyCode::Enter, CTRL_SHIFT, EditorCommand::Ignore),
    (KeyCode::Char('b'), CTRL, EditorCommand::Ignore),
    (KeyCode::Char('b'), CTRL_SHIFT, EditorCommand::Ignore),
];

/// Each key the help lists is pinned once; a repeated row would hide a change.
#[test]
fn each_far_key_is_listed_once() {
    let mut seen = std::collections::HashSet::new();
    for (code, modifiers, _) in UNIMPLEMENTED {
        assert!(
            seen.insert((code, modifiers)),
            "{modifiers:?} {code:?} is listed twice"
        );
    }
}

/// Every row resolves to the command the table pins: nothing, or the language
/// server's hover for Ctrl+K. A key bound to another command later fails here.
#[test]
fn far_editor_keys_without_a_command_stay_unbound() {
    for (code, modifiers, expected) in UNIMPLEMENTED {
        assert_eq!(command(code, modifiers), expected, "{modifiers:?} {code:?}");
    }
}
