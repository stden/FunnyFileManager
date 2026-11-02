//! Purpose: a letter under Ctrl or Alt runs the command its chord is bound to,
//! and a capital under Ctrl is Shift on Unix but `CapsLock` on Windows.

use crossterm::event::{KeyCode, KeyModifiers};

use super::{command, ALT, CTRL, SHIFT};
use crate::action::chord::Platform;
use crate::action::editor::EditorCommand;
use crate::models::LspFeature;

#[test]
fn a_letter_under_ctrl_runs_its_command_and_rejects_shift_and_alt() {
    for (letter, expected) in [
        ('a', EditorCommand::SelectAll),
        ('c', EditorCommand::Copy),
        ('v', EditorCommand::Paste),
        ('k', EditorCommand::Lsp(LspFeature::Hover)),
    ] {
        assert_eq!(command(KeyCode::Char(letter), CTRL), expected);
        for extra in [SHIFT, ALT] {
            assert_eq!(
                command(KeyCode::Char(letter), CTRL.union(extra)),
                EditorCommand::Ignore
            );
        }
    }
}

/// What a capital letter under Ctrl means: on Unix it is Ctrl+Shift, which
/// nothing binds; on Windows it is `CapsLock` and the plain key's command.
fn caps_lock_or_shift(on_windows: EditorCommand) -> EditorCommand {
    match Platform::CURRENT {
        Platform::Unix => EditorCommand::Ignore,
        Platform::Windows => on_windows,
    }
}

/// kitty sends Ctrl+Shift+C as a capital under Ctrl, which is a key of its
/// own; the Windows console sends the capital for `CapsLock`, which is Ctrl+C.
#[test]
fn a_capital_under_ctrl_is_shift_on_unix_and_caps_lock_on_windows() {
    assert_eq!(
        command(KeyCode::Char('C'), CTRL),
        caps_lock_or_shift(EditorCommand::Copy)
    );
    assert_eq!(
        command(KeyCode::Char('F'), CTRL),
        caps_lock_or_shift(EditorCommand::InsertFileName)
    );
    assert_eq!(
        command(KeyCode::Char('L'), CTRL.union(ALT)),
        caps_lock_or_shift(EditorCommand::FormatDocument)
    );
}

#[test]
fn ctrl_alt_l_formats_document_and_rejects_extra_modifiers() {
    let ctrl_alt = KeyModifiers::CONTROL | KeyModifiers::ALT;
    assert_eq!(
        command(KeyCode::Char('l'), ctrl_alt),
        EditorCommand::FormatDocument
    );
    for ch in ['l', 'L'] {
        assert_eq!(
            command(KeyCode::Char(ch), ctrl_alt | KeyModifiers::SHIFT),
            EditorCommand::Ignore
        );
    }
}
