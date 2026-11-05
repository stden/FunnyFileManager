//! Purpose: the `database_explorer` table — Commands → Database explorer: the
//! tree's moves and expansion, the views of a table's rows and columns, and
//! the page keys of the rows.

use crate::action::chord::NamedKey::{Down, Enter, Esc, Left, PgDn, PgUp, Right, Tab, Up};
use crate::action::database::DatabaseCommand;
use crate::keymap::chords::{ch, f, named};
use crate::keymap::Origin::{FarSame, Ffm};
use crate::keymap::{row, Context, Group, Table};

pub const DATABASE_EXPLORER: Table<DatabaseCommand> = Table {
    context: Context::DatabaseExplorer,
    groups: &[TREE, VIEWS],
};

const TREE: Group<DatabaseCommand> = Group {
    title: "The tree",
    rows: &[
        row("cursor_up", DatabaseCommand::Up, &[named(Up)]).note(FarSame, "Move the cursor up"),
        row("cursor_down", DatabaseCommand::Down, &[named(Down)])
            .note(FarSame, "Move the cursor down"),
        row("expand", DatabaseCommand::Expand, &[named(Right)])
            .note(Ffm, "Open the row: schemas, tables, columns"),
        row("collapse", DatabaseCommand::Collapse, &[named(Left)])
            .note(Ffm, "Close the row, or go to its parent"),
        row("open_rows", DatabaseCommand::Open, &[named(Enter)])
            .note(Ffm, "A table's rows; a connection or schema opens"),
        row("columns", DatabaseCommand::Columns, &[named(Tab), ch('c')])
            .note(Ffm, "The columns of the table"),
        row("reload", DatabaseCommand::Reload, &[f(5)])
            .note(Ffm, "Read the connection's metadata again"),
    ],
};

const VIEWS: Group<DatabaseCommand> = Group {
    title: "Rows and columns",
    rows: &[
        row("page_up", DatabaseCommand::PageUp, &[named(PgUp)]).note(Ffm, "The previous 200 rows"),
        row("page_down", DatabaseCommand::PageDown, &[named(PgDn)]).note(Ffm, "The next 200 rows"),
        row("back", DatabaseCommand::Back, &[named(Esc)])
            .note(FarSame, "Back to the tree; from the tree, close"),
        row("close", DatabaseCommand::Close, &[f(10)])
            .note(FarSame, "Close the explorer")
            .exit(),
    ],
};
