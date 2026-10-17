//! Purpose: the `task_list` rows beyond the cursor and the marks: what the
//! list shows, its order, and what is done to the processes.

use crate::action::chord::NamedKey::{Del, Enter, Esc};
use crate::action::tasks::TaskCommand;
use crate::keymap::chords::{ch, ctrl, f, named};
use crate::keymap::Origin::{Far, FarSame, Ffm, Htop};
use crate::keymap::{row, Group};
use crate::models::{NiceStep, SortKey};

use TaskCommand::SortBy;

pub(super) const SHOWING: Group<TaskCommand> = Group {
    title: "What the list shows",
    rows: &[
        row("info", TaskCommand::Info, &[named(Enter), f(3)])
            .note(Far("F3"), "Process information"),
        row("help", TaskCommand::Help, &[f(1), ch('h'), ch('?')])
            .note(FarSame, "The keys of the task list"),
        row(
            "toggle_command_view",
            TaskCommand::ToggleCommandView,
            &[f(2)],
        )
        .note(Ffm, "The command line or the name in the last column"),
        row("filter", TaskCommand::Filter, &[f(4), ch('\\')]).note(Htop("F4"), "Filter the list"),
        row("search", TaskCommand::Search(None), &[ch('/')]).note(Htop("/"), "Search the list"),
        row(
            "toggle_tree",
            TaskCommand::ToggleTree,
            &[f(5), ch('t'), ctrl(ch('t'))],
        )
        .note(Htop("F5"), "The list or the tree of parents"),
        row("user_menu", TaskCommand::UserMenu, &[ch('u')])
            .note(Htop("u"), "Show the processes of one user"),
        row(
            "toggle_kernel_threads",
            TaskCommand::ToggleKernelThreads,
            &[ch('K')],
        )
        .note(Htop("K"), "Show or hide kernel threads"),
        row("toggle_hold", TaskCommand::ToggleHold, &[f(11), ch('Z')])
            .note(Htop("Z"), "Hold or resume the refresh"),
        row(
            "refresh_now",
            TaskCommand::RefreshNow,
            &[ctrl(ch('r')), ctrl(ch('l'))],
        )
        .note(FarSame, "Refresh now"),
    ],
};

pub(super) const SORTING: Group<TaskCommand> = Group {
    title: "Order",
    rows: &[
        row(
            "sort_menu",
            TaskCommand::SortMenu,
            &[f(6), ctrl(f(12)), ch('<'), ch('>')],
        )
        .note(Htop("F6"), "The Sort by menu"),
        row("sort_by_cpu", SortBy(SortKey::Cpu), &[ch('P')]).note(Htop("P"), "Sort by CPU%"),
        row(
            "sort_by_memory",
            SortBy(SortKey::Resident),
            &[ch('M'), ctrl(f(6))],
        )
        .note(Htop("M"), "Sort by memory"),
        row("sort_by_cpu_time", SortBy(SortKey::CpuTime), &[ch('T')])
            .note(Htop("T"), "Sort by CPU time"),
        row("sort_by_pid", SortBy(SortKey::Pid), &[ch('N'), ctrl(f(7))])
            .note(Htop("N"), "Sort by PID"),
        row("sort_by_name", SortBy(SortKey::Name), &[ctrl(f(3))]).note(FarSame, "Sort by name"),
        row(
            "sort_by_start_time",
            SortBy(SortKey::Started),
            &[ctrl(f(8))],
        )
        .note(FarSame, "Sort by start time"),
        row("sort_by_command", SortBy(SortKey::Command), &[ctrl(f(10))])
            .note(FarSame, "Sort by command"),
        row("sort_by_user", SortBy(SortKey::User), &[ctrl(f(11))]).note(FarSame, "Sort by user"),
        row("invert_order", TaskCommand::InvertOrder, &[ch('I')])
            .note(Htop("I"), "Invert the order"),
    ],
};

pub(super) const PROCESSES: Group<TaskCommand> = Group {
    title: "Processes, and closing",
    rows: &[
        row("priority", TaskCommand::Priority, &[f(7)])
            .note(Ffm, "Set the nice value of the main thread"),
        row("nice_raise", TaskCommand::Nice(NiceStep::Raise), &[ch(']')])
            .note(Htop("]"), "Nice -1, at once: runs sooner"),
        row("nice_lower", TaskCommand::Nice(NiceStep::Lower), &[ch('[')])
            .note(Htop("["), "Nice +1: runs later"),
        row("terminate", TaskCommand::Terminate, &[f(8), named(Del)])
            .note(FarSame, "Terminate or kill, after asking"),
        row("signal_menu", TaskCommand::SignalMenu, &[f(9), ch('k')])
            .note(Htop("F9"), "Send a signal, after asking"),
        row(
            "close",
            TaskCommand::Close,
            &[named(Esc), f(10), ch('q'), ctrl(ch('c')), ctrl(ch('w'))],
        )
        .note(FarSame, "Close the task list")
        .exit(),
    ],
};
