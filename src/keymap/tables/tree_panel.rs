//! Purpose: the `tree_panel` table — the Ctrl+T folder tree, read before
//! `panels` while the active panel is a tree.

use crate::action::chord::NamedKey::{Backspace, Enter, Esc, Left, Right};
use crate::action::tree::TreeCommand;
use crate::keymap::chords::{ch, ctrl, named};
use crate::keymap::Origin::{Far, FarSame};
use crate::keymap::{row, Context, Group, Table};

pub const TREE_PANEL: Table<TreeCommand> = Table {
    context: Context::TreePanel,
    groups: &[MOVING, FOLDERS, SEARCH],
};

const MOVING: Group<TreeCommand> = Group {
    title: "Moving around",
    rows: &list_moves!(
        TreeCommand,
        "Go to the first folder",
        "Go to the last folder"
    ),
};

const FOLDERS: Group<TreeCommand> = Group {
    title: "Folders",
    rows: &[
        row("expand", TreeCommand::Expand, &[named(Right)])
            .note(FarSame, "Show the subfolders, or step into the first"),
        row("collapse", TreeCommand::Collapse, &[named(Left)])
            .note(FarSame, "Hide the subfolders, or climb to the parent"),
        row("go_to_folder", TreeCommand::GoToFolder, &[named(Enter)])
            .note(FarSame, "Show the folder's files in the panel"),
        row("next_sibling", TreeCommand::NextSibling, &[ch('+')])
            .note(Far("Gray +"), "The next folder at the same level"),
        row("previous_sibling", TreeCommand::PreviousSibling, &[ch('-')])
            .note(Far("Gray -"), "The previous folder at the same level"),
        row("refresh_tree", TreeCommand::RefreshTree, &[ctrl(ch('r'))])
            .note(FarSame, "Read the tree again"),
    ],
};

const SEARCH: Group<TreeCommand> = Group {
    title: "Search",
    rows: &[
        row("find_next", TreeCommand::FindNext, &[ctrl(named(Enter))])
            .note(FarSame, "The next folder the search matches"),
        row("clear_search", TreeCommand::ClearSearch, &[named(Esc)])
            .note(FarSame, "End the search"),
        row(
            "shorten_search",
            TreeCommand::ShortenSearch,
            &[named(Backspace)],
        )
        .note(FarSame, "Drop the last character of the search"),
    ],
};
