//! Purpose: the `wsl_manager` table — the Ctrl+Alt+W manager of Windows
//! Subsystem for Linux.

use crate::action::chord::NamedKey::{Down, End, Enter, Esc, Home, Up};
use crate::action::wsl::WslCommand;
use crate::keymap::chords::{f, named};
use crate::keymap::Origin::{FarSame, Ffm};
use crate::keymap::{row, Context, Group, Table};

pub const WSL_MANAGER: Table<WslCommand> = Table {
    context: Context::WslManager,
    groups: &[MOVING, COMMANDS],
};

const MOVING: Group<WslCommand> = Group {
    title: "Moving around",
    rows: &[
        row("cursor_up", WslCommand::Up, &[named(Up)]).note(FarSame, "Move the cursor up"),
        row("cursor_down", WslCommand::Down, &[named(Down)]).note(FarSame, "Move the cursor down"),
        row("cursor_home", WslCommand::First, &[named(Home)])
            .note(FarSame, "Go to the first distribution"),
        row("cursor_end", WslCommand::Last, &[named(End)])
            .note(FarSame, "Go to the last distribution"),
    ],
};

const COMMANDS: Group<WslCommand> = Group {
    title: "Distributions",
    rows: &[
        row("open", WslCommand::Open, &[named(Enter)]).note(Ffm, "Open the distribution"),
        row("refresh", WslCommand::Refresh, &[f(2)]).note(Ffm, "Read the list again"),
        row("update_wsl", WslCommand::Update, &[f(5)]).note(Ffm, "Update WSL itself"),
        row("update_packages", WslCommand::UpdatePackages, &[f(6)])
            .note(Ffm, "Update the distribution's packages"),
        row("close", WslCommand::Close, &[named(Esc)])
            .note(FarSame, "Close the manager")
            .exit(),
    ],
};
