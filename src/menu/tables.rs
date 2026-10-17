//! Purpose: the five drop-downs of the menu bar, row for row as Far draws them.

use crate::action::{Action, FileCommand, ToolCommand};

use super::{Entry, MenuCommand};

mod options;
mod panel;

pub(super) use options::OPTIONS;
pub(super) use panel::{PANEL, PANEL_RIGHT};

/// What can be done to the files under the cursor.
///
/// Far's eighteen rows, then a group of this program's own: it has a new-file
/// key and an external editor that Far reaches differently, and a menu that
/// omitted them would be a map with roads missing.
pub(super) const FILES: [Entry; 24] = [
    Entry::item("&View", "F3", Action::View),
    Entry::item("&Edit", "F4", Action::Edit),
    Entry::item("&Copy", "F5", Action::Copy),
    Entry::item("&Rename or move", "F6", Action::Move),
    Entry::item("Lin&k", "Alt+F6", Action::File(FileCommand::Link)),
    Entry::item("&Make folder", "F7", Action::Mkdir),
    Entry::item("&Delete", "F8", Action::DeleteConfirm),
    Entry::item("&Wipe", "Alt+Del", Action::File(FileCommand::Wipe)),
    Entry::Separator,
    Entry::item(
        "Add &to archive",
        "Shift+F1",
        Action::File(FileCommand::AddToArchive),
    ),
    Entry::item(
        "E&xtract files",
        "Shift+F2",
        Action::File(FileCommand::Extract),
    ),
    Entry::item(
        "Arc&hive commands",
        "Shift+F3",
        Action::File(FileCommand::ArchiveCommands),
    ),
    Entry::Separator,
    Entry::item(
        "File &attributes",
        "Ctrl+A",
        Action::File(FileCommand::Attributes),
    ),
    Entry::item(
        "A&pply command",
        "Ctrl+G",
        Action::File(FileCommand::ApplyCommand),
    ),
    Entry::item(
        "Descri&be files",
        "Ctrl+Z",
        Action::File(FileCommand::Describe),
    ),
    Entry::Separator,
    Entry::item("Select &group", "Gray +", Action::SelectGroup),
    Entry::item("U&nselect group", "Gray -", Action::UnselectGroup),
    Entry::item("&Invert selection", "Gray *", Action::InvertSelection),
    Entry::item("Re&store selection", "Ctrl+M", Action::RestoreSelection),
    Entry::Separator,
    Entry::item("New &file", "Shift+F4", Action::NewFile),
    Entry::item("Edit with $EDIT&OR", "Alt+F4", Action::EditExternal),
];

/// Everything that is not about one file.
pub(super) const COMMANDS: &[Entry] = &[
    Entry::item(
        "Action statistics (stats)",
        "",
        Action::Menu(MenuCommand::Performance),
    ),
    Entry::item(
        "Language servers (LSP)",
        "",
        Action::Menu(MenuCommand::LanguageServers),
    ),
    Entry::item(
        "Update pac&kages",
        "",
        Action::Menu(MenuCommand::UpdatePackages),
    ),
    Entry::item(
        "Edit PATH (current user)",
        "",
        Action::Menu(MenuCommand::Path(crate::path_env::Scope::User)),
    ),
    Entry::item(
        "Edit PATH (all users)",
        "",
        Action::Menu(MenuCommand::Path(crate::path_env::Scope::System)),
    ),
    Entry::item("&Find file", "Alt+F7", Action::Tool(ToolCommand::FindFile)),
    Entry::item("&History", "Alt+F8", Action::Tool(ToolCommand::History)),
    Entry::item(
        "&Video mode",
        "Alt+F9",
        Action::Tool(ToolCommand::VideoMode),
    ),
    Entry::item(
        "Fi&nd folder",
        "Alt+F10",
        Action::Tool(ToolCommand::FindFolder),
    ),
    Entry::item(
        "File view histor&y",
        "Alt+F11",
        Action::Tool(ToolCommand::ViewHistory),
    ),
    Entry::item(
        "F&olders history",
        "Alt+F12",
        Action::Tool(ToolCommand::FolderHistory),
    ),
    Entry::Separator,
    Entry::item("&Swap panels", "Ctrl+U", Action::SwapPanels),
    Entry::item("&Panels On/Off", "Ctrl+O", Action::ToggleConsole),
    Entry::item("&Compare folders", "", Action::Menu(MenuCommand::Compare)),
    Entry::Separator,
    Entry::item("Edit user &menu", "", Action::Menu(MenuCommand::UserMenu)),
    Entry::item(
        "File &associations",
        "",
        Action::Menu(MenuCommand::Associations),
    ),
    Entry::item(
        "Fol&der shortcuts",
        "",
        Action::Menu(MenuCommand::Shortcuts),
    ),
    Entry::item(
        "F&ile panel filter",
        "Ctrl+I",
        Action::Tool(ToolCommand::Filter),
    ),
    Entry::Separator,
    Entry::item("Pl&ugin commands", "F11", Action::PluginCommands),
    Entry::item(
        "Install MCP s&erver",
        "",
        Action::Menu(MenuCommand::McpInstall),
    ),
    Entry::item("Sc&reens list", "F12", Action::Tool(ToolCommand::Screens)),
    Entry::item("Task &list", "Ctrl+W", Action::Tool(ToolCommand::Tasks)),
    Entry::item(
        "Ho&tplug devices list",
        "",
        Action::Menu(MenuCommand::Devices),
    ),
    Entry::Separator,
    Entry::item("&Go to root", "Ctrl+\\", Action::GoRoot),
    #[cfg(windows)]
    Entry::item("&WSL manager", "Ctrl+Alt+W", Action::WslManagement),
    Entry::item(
        "Git pull for all repositories",
        "",
        Action::Menu(MenuCommand::GitPullAll),
    ),
];
