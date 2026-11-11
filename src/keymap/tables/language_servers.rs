//! Purpose: the `language_servers` table — Options → Language servers, its
//! list and its F3 view. The view scrolls with the list's moves, checks
//! again with `recheck` and returns with `view` or `close`.

use crate::action::chord::NamedKey::{
    Del, Down, End, Enter, Esc, Home, Ins, PgDn, PgUp, Space, Up,
};
use crate::action::lsp_editor::LspListCommand;
use crate::keymap::chords::{f, named};
use crate::keymap::Origin::{Far, FarSame, Ffm};
use crate::keymap::{row, Context, Group, Table};
use crate::models::lsp_editor::Step;

use LspListCommand::Move;

pub const LANGUAGE_SERVERS: Table<LspListCommand> = Table {
    context: Context::LanguageServers,
    groups: &[MOVING, SERVERS],
};

const MOVING: Group<LspListCommand> = Group {
    title: "Moving around",
    rows: &[
        row("cursor_up", Move(Step::Up), &[named(Up)]).note(FarSame, "Move the cursor up"),
        row("cursor_down", Move(Step::Down), &[named(Down)]).note(FarSame, "Move the cursor down"),
        row("page_up", Move(Step::PageUp), &[named(PgUp)])
            .note(FarSame, "Move the cursor one screen up"),
        row("page_down", Move(Step::PageDown), &[named(PgDn)])
            .note(FarSame, "Move the cursor one screen down"),
        row("cursor_home", Move(Step::First), &[named(Home)])
            .note(FarSame, "Go to the first server"),
        row("cursor_end", Move(Step::Last), &[named(End)]).note(FarSame, "Go to the last server"),
    ],
};

const SERVERS: Group<LspListCommand> = Group {
    title: "Servers",
    rows: &[
        row("edit", LspListCommand::Edit, &[named(Enter), f(4)])
            .note(Far("F4"), "Change the server"),
        row(
            "toggle_enabled",
            LspListCommand::ToggleEnabled,
            &[named(Space)],
        )
        .note(Ffm, "Enable or disable the server"),
        row("add", LspListCommand::Add, &[named(Ins), f(7)]).note(FarSame, "Add a server"),
        row("remove", LspListCommand::Remove, &[named(Del), f(8)])
            .note(FarSame, "Delete the server"),
        row("save", LspListCommand::Save, &[f(2)]).note(FarSame, "Save the servers"),
        row("view", LspListCommand::View, &[f(3)])
            .note(FarSame, "Every server, as text; back to the list"),
        row("recheck", LspListCommand::Recheck, &[f(5)])
            .note(Ffm, "Check every server's command again"),
        row("close", LspListCommand::Close, &[named(Esc), f(10)])
            .note(FarSame, "Close, asking when something changed")
            .exit(),
    ],
};
