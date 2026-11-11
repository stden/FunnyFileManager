//! Purpose: the `command_line` rows that move the caret, select, use the
//! clipboard and change the text — readline's keys, and the selection every
//! field shares.

use crate::action::chord::NamedKey::{Backspace, Del, End, Home, Ins, Left, Right};
use crate::action::command_line::{LineClipboard, LineCommand, LineEdit, LineMotion};
use crate::keymap::chords::{alt, ch, ctrl, named, shift};
use crate::keymap::Origin::{Bash, Ffm};
use crate::keymap::{row, Group};

use LineCommand::{Clipboard, Edit, Move, Select};

pub(super) const MOVING: Group<LineCommand> = Group {
    title: "Moving the caret",
    rows: &[
        row("char_left", Move(LineMotion::CharLeft), &[named(Left)])
            .note(Bash("Left"), "One character left"),
        row("char_right", Move(LineMotion::CharRight), &[named(Right)])
            .note(Bash("Right"), "One character right"),
        row(
            "word_left",
            Move(LineMotion::WordLeft),
            &[ctrl(named(Left)), alt(named(Left)), alt(ch('b'))],
        )
        .note(Bash("Alt+B"), "One word left"),
        row(
            "word_right",
            Move(LineMotion::WordRight),
            &[ctrl(named(Right)), alt(named(Right)), alt(ch('f'))],
        )
        .note(Bash("Alt+F"), "One word right"),
        row(
            "line_start",
            Move(LineMotion::LineStart),
            &[named(Home), ctrl(ch('a'))],
        )
        .note(Bash("Ctrl+A"), "To the start of the line"),
        row(
            "line_end",
            Move(LineMotion::LineEnd),
            &[named(End), ctrl(ch('e'))],
        )
        .note(Bash("Ctrl+E"), "To the end of the line"),
    ],
};

pub(super) const SELECTION: Group<LineCommand> = Group {
    title: "Selecting and the clipboard",
    rows: &[
        row(
            "select_char_left",
            Select(LineMotion::CharLeft),
            &[shift(named(Left))],
        )
        .note(Ffm, "Extend the selection one character left"),
        row(
            "select_char_right",
            Select(LineMotion::CharRight),
            &[shift(named(Right))],
        )
        .note(Ffm, "Extend the selection one character right"),
        row(
            "select_word_left",
            Select(LineMotion::WordLeft),
            &[ctrl(shift(named(Left)))],
        )
        .note(Ffm, "Extend the selection one word left"),
        row(
            "select_word_right",
            Select(LineMotion::WordRight),
            &[ctrl(shift(named(Right)))],
        )
        .note(Ffm, "Extend the selection one word right"),
        row(
            "select_line_start",
            Select(LineMotion::LineStart),
            &[shift(named(Home))],
        )
        .note(Ffm, "Extend the selection to the start of the line"),
        row(
            "select_line_end",
            Select(LineMotion::LineEnd),
            &[shift(named(End))],
        )
        .note(Ffm, "Extend the selection to the end of the line"),
        row("copy", Clipboard(LineClipboard::Copy), &[ctrl(named(Ins))])
            .note(Ffm, "Copy the selection"),
        row(
            "cut",
            Clipboard(LineClipboard::Cut),
            &[ctrl(ch('x')), shift(named(Del))],
        )
        .note(Ffm, "Cut the selection"),
        row(
            "paste",
            Clipboard(LineClipboard::Paste),
            &[ctrl(ch('v')), shift(named(Ins))],
        )
        .note(Ffm, "Paste the clipboard over the selection"),
    ],
};

pub(super) const EDITING: Group<LineCommand> = Group {
    title: "Changing the text",
    rows: &[
        row(
            "kill_word_left",
            Edit(LineEdit::KillWordLeft),
            &[ctrl(ch('w'))],
        )
        .note(Bash("Ctrl+W"), "Cut the word before the caret"),
        row(
            "kill_to_start",
            Edit(LineEdit::KillToStart),
            &[ctrl(ch('u'))],
        )
        .note(Bash("Ctrl+U"), "Cut to the start of the line"),
        row("kill_to_end", Edit(LineEdit::KillToEnd), &[ctrl(ch('k'))])
            .note(Bash("Ctrl+K"), "Cut to the end of the line"),
        row("yank", Edit(LineEdit::Yank), &[ctrl(ch('y'))])
            .note(Bash("Ctrl+Y"), "Put back what was cut"),
        row("backspace", Edit(LineEdit::Backspace), &[named(Backspace)])
            .note(Bash("Backspace"), "Delete the character before the caret"),
        row("delete_char", Edit(LineEdit::DeleteChar), &[named(Del)])
            .note(Bash("Del"), "Delete the character under the caret"),
    ],
};
