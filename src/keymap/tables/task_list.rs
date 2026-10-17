//! Purpose: the task list's two tables — `task_list` while the Ctrl+W list
//! is browsed, `task_list_search` for the key its `/` bar adds. Far's panel
//! keys where they fit a process list, htop's letters elsewhere.

use crate::action::chord::NamedKey::{Ins, Left, Right, Space};
use crate::action::tasks::search::TaskSearchCommand;
use crate::action::tasks::{Pan, PanStep, TaskCommand};
use crate::keymap::chords::{ch, ctrl, f, named};
use crate::keymap::Origin::{Far, Ffm, Htop};
use crate::keymap::{row, Context, Group, Table};
use crate::models::MaskPurpose;

mod commands;

pub const TASK_LIST: Table<TaskCommand> = Table {
    context: Context::TaskList,
    groups: &[
        MOVING,
        MARKING,
        commands::SHOWING,
        commands::SORTING,
        commands::PROCESSES,
    ],
};

pub const TASK_LIST_SEARCH: Table<TaskSearchCommand> = Table {
    context: Context::TaskListSearch,
    groups: &[SEARCH_BAR],
};

const SEARCH_BAR: Group<TaskSearchCommand> = Group {
    title: "Search bar",
    rows: &[row("find_next", TaskSearchCommand::FindNext, &[f(3)])
        .note(Htop("F3"), "The next match after the cursor")],
};

const MOVING: Group<TaskCommand> = Group {
    title: "Moving around",
    rows: &list_moves!(
        TaskCommand,
        "Go to the first process",
        "Go to the last process",
        [
            row(
                "scroll_left",
                TaskCommand::Pan(Pan::Left(PanStep::Cell)),
                &[named(Left)],
            )
            .note(Htop("Left"), "Scroll the Command column left"),
            row(
                "scroll_right",
                TaskCommand::Pan(Pan::Right(PanStep::Cell)),
                &[named(Right)],
            )
            .note(Htop("Right"), "Scroll the Command column right"),
            row(
                "scroll_left_wide",
                TaskCommand::Pan(Pan::Left(PanStep::Wide)),
                &[ctrl(named(Left))],
            )
            .note(Ffm, "Scroll the Command column twenty columns left"),
            row(
                "scroll_right_wide",
                TaskCommand::Pan(Pan::Right(PanStep::Wide)),
                &[ctrl(named(Right))],
            )
            .note(Ffm, "Scroll the Command column twenty columns right"),
        ],
    ),
};

const MARKING: Group<TaskCommand> = Group {
    title: "Marking",
    rows: &[
        row(
            "toggle_mark",
            TaskCommand::ToggleMark,
            &[named(Space), named(Ins)],
        )
        .note(Far("Ins"), "Mark or unmark the process, then move down"),
        row("mark_family", TaskCommand::MarkFamily, &[ch('c')])
            .note(Htop("c"), "Mark the process and its descendants"),
        row("unmark_all", TaskCommand::UnmarkAll, &[ch('U')]).note(Htop("U"), "Unmark all"),
        row("invert_marks", TaskCommand::InvertMarks, &[ch('*')])
            .note(Far("Gray *"), "Invert the marks"),
        row(
            "select_by_mask",
            TaskCommand::MarkByMask(MaskPurpose::Select),
            &[ch('+')],
        )
        .note(Far("Gray +"), "Mark processes by a name mask"),
        row(
            "unselect_by_mask",
            TaskCommand::MarkByMask(MaskPurpose::Unselect),
            &[ch('-')],
        )
        .note(Far("Gray -"), "Unmark processes by a name mask"),
    ],
};
