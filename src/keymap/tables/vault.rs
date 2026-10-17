//! Purpose: the `vault` table — the `KeePass` vault while it is browsed,
//! locked or unlocked, and the command keys of its entry form (`save`,
//! `undo`). In a build without `KeePass` nothing reads it.

use crate::action::chord::NamedKey::{Backspace, Enter, Esc};
use crate::action::vault::VaultCommand;
use crate::keymap::chords::{ch, ctrl, f, named};
use crate::keymap::Origin::{FarSame, Ffm};
use crate::keymap::{row, Context, Group, Table};

pub const VAULT: Table<VaultCommand> = Table {
    context: Context::Vault,
    groups: &[MOVING, ENTRIES, VAULT_ITSELF],
};

const MOVING: Group<VaultCommand> = Group {
    title: "Moving around",
    rows: &list_moves!(
        VaultCommand,
        "Go to the first entry",
        "Go to the last entry"
    ),
};

const ENTRIES: Group<VaultCommand> = Group {
    title: "Entries",
    rows: &[
        row("activate", VaultCommand::Activate, &[named(Enter)])
            .note(Ffm, "Unlock the vault, or copy the password"),
        row(
            "copy_username",
            VaultCommand::CopyUsername,
            &[ctrl(ch('u'))],
        )
        .note(Ffm, "Copy the user name"),
        row("reveal", VaultCommand::Reveal, &[ctrl(ch('r'))]).note(Ffm, "Show the password"),
        row("undo", VaultCommand::Undo, &[ctrl(ch('z'))])
            .note(Ffm, "Undo the last change, in the list or the form"),
        row("edit", VaultCommand::Edit, &[f(4)]).note(FarSame, "Change the entry"),
        row("save", VaultCommand::Save, &[f(2)]).note(FarSame, "Save the entry form"),
        row("toggle_view", VaultCommand::ToggleView, &[f(6)])
            .note(Ffm, "The entries as a list or as a tree"),
        row("delete", VaultCommand::Delete, &[f(8)])
            .note(FarSame, "Delete the entry, after asking"),
        row("backspace", VaultCommand::Backspace, &[named(Backspace)])
            .note(Ffm, "Shorten the password or the filter"),
    ],
};

const VAULT_ITSELF: Group<VaultCommand> = Group {
    title: "Locking and closing",
    rows: &[
        row("lock", VaultCommand::Lock, &[ctrl(ch('l'))]).note(Ffm, "Forget every key and close"),
        row("lock_and_quit", VaultCommand::LockAndQuit, &[ctrl(ch('c'))])
            .note(Ffm, "Lock, close, and quit"),
        row("close", VaultCommand::Close, &[named(Esc)])
            .note(FarSame, "Remember the session and close")
            .exit(),
    ],
};
