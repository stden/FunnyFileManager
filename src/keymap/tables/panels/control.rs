//! Purpose: the `panels` rows of the Ctrl keys: Ctrl with a function key
//! (the panels on and off, the sort modes), with a digit (the view modes)
//! and with a letter (panel kinds, file commands, the program's windows).

use crate::action::{Action, FileCommand, PanelCommand, ToolCommand};
use crate::keymap::chords::{alt, ch, ctrl, f};
use crate::keymap::Origin::{FarSame, Ffm};
use crate::keymap::{row, Group};
use crate::models::{PanelSide, PanelView as View, SortMode};

use Action::{File, Panel, PanelView, Tool};
use PanelCommand::{Sort, TogglePanel};

pub(super) const PANELS_AND_SORTING: Group<Action> = Group {
    title: "Panels on and off, sort modes",
    rows: &[
        row(
            "toggle_left_panel",
            Panel(TogglePanel(PanelSide::Left)),
            &[ctrl(f(1))],
        )
        .note(FarSame, "Show or hide the left panel"),
        row(
            "toggle_right_panel",
            Panel(TogglePanel(PanelSide::Right)),
            &[ctrl(f(2))],
        )
        .note(FarSame, "Show or hide the right panel"),
        row("sort_by_name", Panel(Sort(SortMode::Name)), &[ctrl(f(3))])
            .note(FarSame, "Sort by name"),
        row(
            "sort_by_extension",
            Panel(Sort(SortMode::Extension)),
            &[ctrl(f(4))],
        )
        .note(FarSame, "Sort by extension"),
        row(
            "sort_by_modification_time",
            Panel(Sort(SortMode::ModificationTime)),
            &[ctrl(f(5))],
        )
        .note(FarSame, "Sort by modification time"),
        row("sort_by_size", Panel(Sort(SortMode::Size)), &[ctrl(f(6))])
            .note(FarSame, "Sort by size"),
        row("unsorted", Panel(Sort(SortMode::Unsorted)), &[ctrl(f(7))])
            .note(FarSame, "The folder's own order"),
        row(
            "sort_by_creation_time",
            Panel(Sort(SortMode::CreationTime)),
            &[ctrl(f(8))],
        )
        .note(FarSame, "Sort by creation time"),
        row(
            "sort_by_access_time",
            Panel(Sort(SortMode::AccessTime)),
            &[ctrl(f(9))],
        )
        .note(FarSame, "Sort by access time"),
        row(
            "sort_by_description",
            Panel(Sort(SortMode::Descriptions)),
            &[ctrl(f(10))],
        )
        .note(FarSame, "Sort by description"),
        row(
            "sort_by_owner",
            Panel(Sort(SortMode::Owner)),
            &[ctrl(f(11))],
        )
        .note(FarSame, "Sort by owner"),
        row("sort_menu", Panel(PanelCommand::SortMenu), &[ctrl(f(12))])
            .note(FarSame, "The sort modes menu"),
    ],
};

pub(super) const VIEW_MODES: Group<Action> = Group {
    title: "View modes",
    rows: &[
        row("view_brief", PanelView(View::Brief), &[ctrl(ch('1'))]).note(FarSame, "Brief view"),
        row("view_medium", PanelView(View::Medium), &[ctrl(ch('2'))]).note(FarSame, "Medium view"),
        row("view_full", PanelView(View::Full), &[ctrl(ch('3'))]).note(FarSame, "Full view"),
        row("view_wide", PanelView(View::Wide), &[ctrl(ch('4'))]).note(FarSame, "Wide view"),
        row("view_detailed", PanelView(View::Detailed), &[ctrl(ch('5'))])
            .note(FarSame, "Detailed view"),
        row(
            "view_descriptions",
            PanelView(View::Descriptions),
            &[ctrl(ch('6'))],
        )
        .note(FarSame, "Descriptions view"),
        row(
            "view_long_descriptions",
            PanelView(View::LongDescriptions),
            &[ctrl(ch('7'))],
        )
        .note(FarSame, "Long descriptions view"),
        row("view_owners", PanelView(View::Owners), &[ctrl(ch('8'))])
            .note(FarSame, "File owners view"),
        row("view_links", PanelView(View::Links), &[ctrl(ch('9'))])
            .note(FarSame, "File links view"),
        row(
            "view_alternative_full",
            PanelView(View::Alternative),
            &[ctrl(ch('0'))],
        )
        .note(FarSame, "Alternative full view"),
    ],
};

pub(super) const COMMANDS: Group<Action> = Group {
    title: "Panel kinds, file commands and tools",
    rows: &[
        row("information", Action::Information, &[ctrl(ch('l'))])
            .note(FarSame, "Measure the folder and show what is in it"),
        row("tree_panel", Panel(PanelCommand::Tree), &[ctrl(ch('t'))])
            .note(FarSame, "The panel as a folder tree, or back"),
        row(
            "quick_view",
            Panel(PanelCommand::QuickView),
            &[ctrl(ch('q'))],
        )
        .note(FarSame, "Preview the other panel's entry, or back"),
        row(
            "long_names",
            Panel(PanelCommand::LongNames),
            &[ctrl(ch('n'))],
        )
        .note(FarSame, "Long names or their short form"),
        row(
            "attributes",
            File(FileCommand::Attributes),
            &[ctrl(ch('a'))],
        )
        .note(FarSame, "Permissions and times of the selection"),
        row(
            "apply_command",
            File(FileCommand::ApplyCommand),
            &[ctrl(ch('g'))],
        )
        .note(FarSame, "Run a command once for each selected entry"),
        row("describe", File(FileCommand::Describe), &[ctrl(ch('z'))])
            .note(FarSame, "Describe the selected entries"),
        row("filter", Tool(ToolCommand::Filter), &[ctrl(ch('i'))])
            .note(FarSame, "The file panel filter"),
        row("task_list", Tool(ToolCommand::Tasks), &[ctrl(ch('w'))])
            .note(FarSame, "The task list: processes"),
        row("toggle_console", Action::ToggleConsole, &[ctrl(ch('o'))])
            .note(FarSame, "Hide the panels for the console, or back")
            .exit(),
        row("refresh", Action::Refresh, &[ctrl(ch('r'))]).note(FarSame, "Read both panels again"),
        row("quit", Action::Quit, &[ctrl(ch('c'))]).note(Ffm, "Quit at once, without asking"),
        row("wsl_manager", Action::WslManagement, &[ctrl(alt(ch('w')))])
            .note(Ffm, "Manage Windows Subsystem for Linux"),
    ],
};
