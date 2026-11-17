//! Purpose: the `path_editor` table — Commands → Edit PATH, while its list
//! is browsed.

use crate::action::chord::NamedKey::{Del, Down, End, Enter, Esc, Home, Ins, Up};
use crate::action::path_env::PathCommand;
use crate::keymap::chords::{ctrl, f, named};
use crate::keymap::Origin::{Far, FarSame, Ffm};
use crate::keymap::{row, Context, Group, Table};

pub const PATH_EDITOR: Table<PathCommand> = Table {
    context: Context::PathEditor,
    groups: &[MOVING, ENTRIES],
};

const MOVING: Group<PathCommand> = Group {
    title: "Moving around",
    rows: &[
        row("cursor_up", PathCommand::CursorUp, &[named(Up)]).note(FarSame, "Move the cursor up"),
        row("cursor_down", PathCommand::CursorDown, &[named(Down)])
            .note(FarSame, "Move the cursor down"),
        row("cursor_home", PathCommand::CursorHome, &[named(Home)])
            .note(FarSame, "Go to the first entry"),
        row("cursor_end", PathCommand::CursorEnd, &[named(End)])
            .note(FarSame, "Go to the last entry"),
    ],
};

const ENTRIES: Group<PathCommand> = Group {
    title: "Entries",
    rows: &[
        row(
            "move_entry_up",
            PathCommand::MoveEntryUp,
            &[ctrl(named(Up))],
        )
        .note(Ffm, "Move the entry one row up"),
        row(
            "move_entry_down",
            PathCommand::MoveEntryDown,
            &[ctrl(named(Down))],
        )
        .note(Ffm, "Move the entry one row down"),
        row("add", PathCommand::Add, &[named(Ins), f(7)]).note(FarSame, "Add an entry"),
        row("edit", PathCommand::Edit, &[named(Enter), f(4)]).note(Far("F4"), "Change the entry"),
        row("remove", PathCommand::Remove, &[named(Del), f(8)]).note(FarSame, "Drop the entry"),
        row("recheck", PathCommand::Recheck, &[f(5)]).note(Ffm, "Check every entry again"),
        row("save", PathCommand::Save, &[f(2)]).note(FarSame, "Save, after asking"),
        row("close", PathCommand::Close, &[named(Esc)])
            .note(FarSame, "Close, asking when something changed")
            .exit(),
    ],
};
