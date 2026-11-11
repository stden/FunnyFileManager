//! Purpose: the `panels` rows of the keys without a modifier family — the
//! cursor, marking, the command line's opener and the quick search.

use crate::action::chord::NamedKey::{
    Backspace, Down, End, Enter, Home, Ins, Left, PgDn, PgUp, Right, Space, Tab, Up,
};
use crate::action::Action;
use crate::keymap::chords::{ch, ctrl, named};
use crate::keymap::Origin::{Far, FarSame, Ffm};
use crate::keymap::{row, Group};
use crate::models::PanelSide;

pub(super) const MOVING: Group<Action> = Group {
    title: "Moving around",
    rows: &[
        row("cursor_up", Action::NavUp, &[named(Up)]).note(FarSame, "Move the cursor up"),
        row("cursor_down", Action::NavDown, &[named(Down)]).note(FarSame, "Move the cursor down"),
        row("page_up", Action::PageUp, &[named(PgUp)])
            .note(FarSame, "Move the cursor one screen up"),
        row("page_down", Action::PageDown, &[named(PgDn)])
            .note(FarSame, "Move the cursor one screen down"),
        row("cursor_home", Action::First, &[named(Home)]).note(FarSame, "Go to the first entry"),
        row("cursor_end", Action::Last, &[named(End)]).note(FarSame, "Go to the last entry"),
        row("open", Action::Open, &[named(Enter)])
            .note(FarSame, "Enter the folder or run the file"),
        row("switch_panel", Action::SwitchPanel, &[named(Tab)])
            .note(FarSame, "Switch to the other panel"),
        row(
            "select_left_panel",
            Action::SelectPanel(PanelSide::Left),
            &[named(Left)],
        )
        .note(FarSame, "Make the left panel active, or move a column left"),
        row(
            "select_right_panel",
            Action::SelectPanel(PanelSide::Right),
            &[named(Right)],
        )
        .note(
            FarSame,
            "Make the right panel active, or move a column right",
        ),
        row("go_root", Action::GoRoot, &[ctrl(ch('\\'))])
            .note(FarSame, "Go to the root of the drive"),
        row("swap_panels", Action::SwapPanels, &[ctrl(ch('u'))]).note(FarSame, "Swap the panels"),
    ],
};

pub(super) const MARKING: Group<Action> = Group {
    title: "Marking",
    rows: &[
        row(
            "toggle_mark",
            Action::ToggleMark,
            &[named(Ins), named(Space)],
        )
        .note(FarSame, "Mark or unmark, then move down"),
        row("select_group", Action::SelectGroup, &[ch('+')])
            .note(Far("Gray +"), "Select files by a mask"),
        row("unselect_group", Action::UnselectGroup, &[ch('-')])
            .note(Far("Gray -"), "Unselect files by a mask"),
        row("invert_selection", Action::InvertSelection, &[ch('*')])
            .note(Far("Gray *"), "Invert the selection"),
        row(
            "restore_selection",
            Action::RestoreSelection,
            &[ctrl(ch('m'))],
        )
        .note(FarSame, "Put the selection back as it was"),
    ],
};

pub(super) const COMMAND_LINE: Group<Action> = Group {
    title: "Command line and quick search",
    rows: &[
        row("command_line", Action::CommandLine, &[ch(':')]).note(Ffm, "Open the command line"),
        row(
            "quick_search_backspace",
            Action::QuickSearchBackspace,
            &[named(Backspace)],
        )
        .note(FarSame, "Drop the last character of the quick search"),
    ],
};
