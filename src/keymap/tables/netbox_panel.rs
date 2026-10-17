//! Purpose: the `netbox_panel` table — the `NetBox` sessions list and a
//! remote panel, read before `panels` while the active panel is one of
//! them. The session keys are the ones Far's `NetBox` gives a session.

use crate::action::chord::NamedKey::Enter;
use crate::action::netbox::{NetboxCommand, RowCommand};
use crate::keymap::chords::{f, named, shift};
use crate::keymap::Origin::FarSame;
use crate::keymap::{row, Context, Group, Table};

use NetboxCommand::Row;

pub const NETBOX_PANEL: Table<NetboxCommand> = Table {
    context: Context::NetboxPanel,
    groups: &[MOVING, SESSIONS],
};

const MOVING: Group<NetboxCommand> = Group {
    title: "Moving around",
    rows: &list_moves!(NetboxCommand, "Go to the first row", "Go to the last row"),
};

const SESSIONS: Group<NetboxCommand> = Group {
    title: "Sessions",
    rows: &[
        row("open", Row(RowCommand::Open), &[named(Enter)])
            .note(FarSame, "Connect, enter the folder, or leave on the .. row"),
        row("view_session", Row(RowCommand::ViewSession), &[f(3)])
            .note(FarSame, "Show the session"),
        row("edit_session", Row(RowCommand::EditSession), &[f(4)])
            .note(FarSame, "Change the session"),
        row("new_session", NetboxCommand::NewSession, &[shift(f(4))])
            .note(FarSame, "Make a session"),
        row("copy_session", Row(RowCommand::CopySession), &[f(5)])
            .note(FarSame, "Copy the session under a new name"),
        row("delete_session", Row(RowCommand::DeleteSession), &[f(8)])
            .note(FarSame, "Delete the session, after asking"),
    ],
};
