//! Unit tests: every default key of the command line resolves to its
//! command and, with a modifier more, to nothing; the keys a file gives are
//! obeyed; a key nothing is bound to types its character or does nothing.

use super::*;
use crate::keymap::Keymap;
use crate::test_support::chord;

use LineCommand::{Clipboard, Edit, Move, Select};

/// Every default key beside its command, as `ffm.yml` spells the key.
const DEFAULTS: [(&str, LineCommand); 37] = [
    ("Up", LineCommand::HistoryBack),
    ("Down", LineCommand::HistoryForward),
    ("Ctrl+R", LineCommand::HistorySearch),
    ("Tab", LineCommand::Complete),
    ("Enter", LineCommand::Submit),
    ("Esc", LineCommand::Cancel),
    ("Ctrl+C", LineCommand::CopyOrClear),
    ("Ctrl+D", LineCommand::DeleteCharOrClose),
    ("Left", Move(LineMotion::CharLeft)),
    ("Right", Move(LineMotion::CharRight)),
    ("Ctrl+Left", Move(LineMotion::WordLeft)),
    ("Alt+Left", Move(LineMotion::WordLeft)),
    ("Alt+B", Move(LineMotion::WordLeft)),
    ("Ctrl+Right", Move(LineMotion::WordRight)),
    ("Alt+Right", Move(LineMotion::WordRight)),
    ("Alt+F", Move(LineMotion::WordRight)),
    ("Home", Move(LineMotion::LineStart)),
    ("Ctrl+A", Move(LineMotion::LineStart)),
    ("End", Move(LineMotion::LineEnd)),
    ("Ctrl+E", Move(LineMotion::LineEnd)),
    ("Shift+Left", Select(LineMotion::CharLeft)),
    ("Shift+Right", Select(LineMotion::CharRight)),
    ("Ctrl+Shift+Left", Select(LineMotion::WordLeft)),
    ("Ctrl+Shift+Right", Select(LineMotion::WordRight)),
    ("Shift+Home", Select(LineMotion::LineStart)),
    ("Shift+End", Select(LineMotion::LineEnd)),
    ("Ctrl+Ins", Clipboard(LineClipboard::Copy)),
    ("Ctrl+X", Clipboard(LineClipboard::Cut)),
    ("Shift+Del", Clipboard(LineClipboard::Cut)),
    ("Ctrl+V", Clipboard(LineClipboard::Paste)),
    ("Shift+Ins", Clipboard(LineClipboard::Paste)),
    ("Ctrl+W", Edit(LineEdit::KillWordLeft)),
    ("Ctrl+U", Edit(LineEdit::KillToStart)),
    ("Ctrl+K", Edit(LineEdit::KillToEnd)),
    ("Ctrl+Y", Edit(LineEdit::Yank)),
    ("Backspace", Edit(LineEdit::Backspace)),
    ("Del", Edit(LineEdit::DeleteChar)),
];

/// Keys that ran a command while a modifier nobody named was ignored: the
/// selection and clipboard keys took Alt and Ctrl+Shift as well.
const A_MODIFIER_MORE: [&str; 13] = [
    "Shift+Enter",
    "Ctrl+Up",
    "Alt+Down",
    "Shift+Backspace",
    "Ctrl+Del",
    "Alt+Shift+Left",
    "Ctrl+End",
    "Alt+Tab",
    "Shift+Esc",
    "Ctrl+Shift+R",
    "Ctrl+Shift+Ins",
    "Ctrl+Alt+A",
    "Ctrl+Alt+B",
];

/// The event a terminal sends for the chord `text` writes.
fn press(text: &str) -> KeyEvent {
    chord(text).to_event()
}

fn by_default(key: KeyEvent) -> Option<LineCommand> {
    resolve(&Keymap::default().command_line, key)
}

#[test]
fn every_default_key_resolves_to_its_command() {
    for (text, command) in DEFAULTS {
        assert_eq!(by_default(press(text)), Some(command), "{text}");
    }
    assert_eq!(Keymap::default().command_line.chords().count(), 37);
}

#[test]
fn a_modifier_more_is_another_key() {
    for text in A_MODIFIER_MORE {
        assert_eq!(by_default(press(text)), None, "{text}");
    }
}

#[test]
fn shift_extends_the_selection_and_ctrl_shift_moves_it_by_word() {
    for (text, motion) in [
        ("Shift+Left", LineMotion::CharLeft),
        ("Shift+Right", LineMotion::CharRight),
        ("Shift+Home", LineMotion::LineStart),
        ("Shift+End", LineMotion::LineEnd),
        ("Ctrl+Shift+Left", LineMotion::WordLeft),
        ("Ctrl+Shift+Right", LineMotion::WordRight),
    ] {
        assert_eq!(by_default(press(text)), Some(Select(motion)), "{text}");
    }
}

#[test]
fn the_clipboard_keys_copy_cut_and_paste() {
    for (text, operation) in [
        ("Ctrl+Ins", LineClipboard::Copy),
        ("Ctrl+X", LineClipboard::Cut),
        ("Shift+Del", LineClipboard::Cut),
        ("Ctrl+V", LineClipboard::Paste),
        ("Shift+Ins", LineClipboard::Paste),
    ] {
        assert_eq!(
            by_default(press(text)),
            Some(Clipboard(operation)),
            "{text}"
        );
    }
}

#[test]
fn the_plain_keys_keep_their_meaning_beside_the_selection_keys() {
    assert_eq!(by_default(press("Left")), Some(Move(LineMotion::CharLeft)));
    assert_eq!(by_default(press("Del")), Some(Edit(LineEdit::DeleteChar)));
    assert_eq!(by_default(press("Home")), Some(Move(LineMotion::LineStart)));
    assert_eq!(by_default(press("Ctrl+C")), Some(LineCommand::CopyOrClear));
}

mod fallback;
