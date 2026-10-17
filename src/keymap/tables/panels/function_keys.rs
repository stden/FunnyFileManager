//! Purpose: the `panels` rows of the function keys: bare, with Shift and
//! with Alt (Alt+Del beside them, as the Files menu has it).

use crate::action::chord::NamedKey::Del;
use crate::action::{Action, FileCommand, ToolCommand};
use crate::keymap::chords::{alt, f, named, shift};
use crate::keymap::Origin::FarSame;
use crate::keymap::{row, Group};
use crate::menu::MenuCommand;
use crate::models::PanelSide;

use Action::{File, Tool};

pub(super) const BARE: Group<Action> = Group {
    title: "Function keys",
    rows: &[
        row("toggle_help", Action::ToggleHelp, &[f(1)]).note(FarSame, "Show or hide the help"),
        row("user_menu", Action::UserMenu, &[f(2)]).note(FarSame, "Open the user menu"),
        row("view", Action::View, &[f(3)]).note(FarSame, "View the file under the cursor"),
        row("edit", Action::Edit, &[f(4)]).note(FarSame, "Edit the file under the cursor"),
        row("copy", Action::Copy, &[f(5)]).note(FarSame, "Copy to the other panel"),
        row("move", Action::Move, &[f(6)]).note(FarSame, "Rename or move to the other panel"),
        row("mkdir", Action::Mkdir, &[f(7)]).note(FarSame, "Make a folder"),
        row("delete", Action::DeleteConfirm, &[f(8)]).note(FarSame, "Delete, after asking"),
        row("menu_bar", Action::MenuBar, &[f(9)]).note(FarSame, "Open the menu bar"),
        row("confirm_quit", Action::ConfirmQuit, &[f(10)])
            .note(FarSame, "Quit, after asking")
            .exit(),
        row("plugin_commands", Action::PluginCommands, &[f(11)])
            .note(FarSame, "Plugin commands: MCP status and NetBox sessions"),
        row("screens", Tool(ToolCommand::Screens), &[f(12)])
            .note(FarSame, "List the screens: the panels and the console"),
    ],
};

pub(super) const SHIFT: Group<Action> = Group {
    title: "Shift and a function key",
    rows: &[
        row(
            "add_to_archive",
            File(FileCommand::AddToArchive),
            &[shift(f(1))],
        )
        .note(FarSame, "Add the selection to an archive"),
        row("extract_files", File(FileCommand::Extract), &[shift(f(2))])
            .note(FarSame, "Extract the selected archives"),
        row(
            "archive_commands",
            File(FileCommand::ArchiveCommands),
            &[shift(f(3))],
        )
        .note(FarSame, "Archive commands: test, list, extract"),
        row("new_file", Action::NewFile, &[shift(f(4))]).note(FarSame, "Create a file and edit it"),
        row("rename", Action::Rename, &[shift(f(6))])
            .note(FarSame, "Rename or move the entry in place"),
        row(
            "save_setup",
            Action::Menu(MenuCommand::SaveSetup),
            &[shift(f(9))],
        )
        .note(FarSame, "Save the settings"),
    ],
};

pub(super) const ALT: Group<Action> = Group {
    title: "Alt and a function key",
    rows: &[
        row(
            "drive_menu_left",
            Action::DriveMenu(PanelSide::Left),
            &[alt(f(1))],
        )
        .note(FarSame, "Change the left panel's drive"),
        row(
            "drive_menu_right",
            Action::DriveMenu(PanelSide::Right),
            &[alt(f(2))],
        )
        .note(FarSame, "Change the right panel's drive"),
        row("edit_external", Action::EditExternal, &[alt(f(4))])
            .note(FarSame, "Edit the file in the external editor"),
        row("link", File(FileCommand::Link), &[alt(f(6))])
            .note(FarSame, "Make a symbolic or hard link"),
        row("find_file", Tool(ToolCommand::FindFile), &[alt(f(7))]).note(FarSame, "Find files"),
        row("command_history", Tool(ToolCommand::History), &[alt(f(8))])
            .note(FarSame, "The commands run, as a list"),
        row("video_mode", Tool(ToolCommand::VideoMode), &[alt(f(9))])
            .note(FarSame, "Switch between 25 and 50 rows"),
        row("find_folder", Tool(ToolCommand::FindFolder), &[alt(f(10))])
            .note(FarSame, "Find a folder"),
        row(
            "view_history",
            Tool(ToolCommand::ViewHistory),
            &[alt(f(11))],
        )
        .note(FarSame, "The files viewed and edited"),
        row(
            "folder_history",
            Tool(ToolCommand::FolderHistory),
            &[alt(f(12))],
        )
        .note(FarSame, "The folders visited"),
        row("wipe", File(FileCommand::Wipe), &[alt(named(Del))])
            .note(FarSame, "Overwrite, then delete"),
    ],
};
