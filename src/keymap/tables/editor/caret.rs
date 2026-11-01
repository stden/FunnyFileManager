//! Purpose: the `editor` rows that move the caret, and the same moves with
//! Shift, which select.

use crate::action::chord::NamedKey::{Down, End, Home, Left, PgDn, PgUp, Right, Up};
use crate::action::editor::{EditorCommand, EditorNavigation as To};
use crate::keymap::chords::{ctrl, named, shift};
use crate::keymap::Origin::FarSame;
use crate::keymap::{row, Group};
use crate::models::SelectionMode;

/// The caret moves and any selection ends.
const fn go(motion: To) -> EditorCommand {
    EditorCommand::Move {
        motion,
        selection: SelectionMode::Move,
    }
}

/// The caret moves and the selection follows it.
const fn select(motion: To) -> EditorCommand {
    EditorCommand::Move {
        motion,
        selection: SelectionMode::Extend,
    }
}

pub(super) const MOVING: Group<EditorCommand> = Group {
    title: "Moving the caret",
    rows: &[
        row("cursor_left", go(To::Left), &[named(Left)]).note(FarSame, "One character left"),
        row("cursor_right", go(To::Right), &[named(Right)]).note(FarSame, "One character right"),
        row("cursor_up", go(To::Up), &[named(Up)]).note(FarSame, "One line up"),
        row("cursor_down", go(To::Down), &[named(Down)]).note(FarSame, "One line down"),
        row("line_start", go(To::Home), &[named(Home)]).note(FarSame, "To the start of the line"),
        row("line_end", go(To::End), &[named(End)]).note(FarSame, "To the end of the line"),
        row("page_up", go(To::PageUp), &[named(PgUp)])
            .note(FarSame, "One page up (a fixed 20 lines, not the screen)"),
        row("page_down", go(To::PageDown), &[named(PgDn)])
            .note(FarSame, "One page down (a fixed 20 lines, not the screen)"),
        row(
            "document_start",
            go(To::DocumentStart),
            &[ctrl(named(Home)), ctrl(named(PgUp))],
        )
        .note(FarSame, "To the start of the document"),
        row(
            "document_end",
            go(To::DocumentEnd),
            &[ctrl(named(End)), ctrl(named(PgDn))],
        )
        .note(FarSame, "To the end of the document"),
    ],
};

pub(super) const SELECTING: Group<EditorCommand> = Group {
    title: "Selecting",
    rows: &[
        row("select_left", select(To::Left), &[shift(named(Left))])
            .note(FarSame, "Select one character left"),
        row("select_right", select(To::Right), &[shift(named(Right))])
            .note(FarSame, "Select one character right"),
        row("select_up", select(To::Up), &[shift(named(Up))]).note(FarSame, "Select one line up"),
        row("select_down", select(To::Down), &[shift(named(Down))])
            .note(FarSame, "Select one line down"),
        row("select_line_start", select(To::Home), &[shift(named(Home))])
            .note(FarSame, "Select to the start of the line"),
        row("select_line_end", select(To::End), &[shift(named(End))])
            .note(FarSame, "Select to the end of the line"),
        row("select_page_up", select(To::PageUp), &[shift(named(PgUp))]).note(
            FarSame,
            "Select one page up (a fixed 20 lines, not the screen)",
        ),
        row(
            "select_page_down",
            select(To::PageDown),
            &[shift(named(PgDn))],
        )
        .note(
            FarSame,
            "Select one page down (a fixed 20 lines, not the screen)",
        ),
        row(
            "select_document_start",
            select(To::DocumentStart),
            &[shift(ctrl(named(Home)))],
        )
        .note(FarSame, "Select to the start of the document"),
        row(
            "select_document_end",
            select(To::DocumentEnd),
            &[shift(ctrl(named(End)))],
        )
        .note(FarSame, "Select to the end of the document"),
    ],
};

#[cfg(test)]
mod tests;
