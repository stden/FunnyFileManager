//! Purpose: the five drop-downs of the menu bar, row for row as Far draws them.

use crossterm::event::KeyCode;
#[cfg(windows)]
use crossterm::event::KeyModifiers;

use super::{Entry, MenuCommand, Shortcut};

mod options;

pub(super) use options::OPTIONS;

/// A panel's own menu, row for row as Far draws it.
///
/// Ten view modes followed by panel tools and settings.
/// The renderer checks the active mode and current panel options.
pub(super) const PANEL: [Entry; 20] =
    panel_menu(crate::models::PanelSide::Left, "Ctrl+F1", "Alt+F1", 1);
/// The same, aimed at the right panel: only its toggle and drive keys differ
/// — Far gives each side its own.
pub(super) const PANEL_RIGHT: [Entry; 20] =
    panel_menu(crate::models::PanelSide::Right, "Ctrl+F2", "Alt+F2", 2);

/// The Left or Right menu for `side`, whose Panel On/Off key is `toggle` and
/// whose Change drive key is `drive` (Alt+F`drive_key`).
const fn panel_menu(
    side: crate::models::PanelSide,
    toggle: &'static str,
    drive: &'static str,
    drive_key: u8,
) -> [Entry; 20] {
    [
        Entry::item("&Brief", "Ctrl+1", Shortcut::ctrl('1')),
        Entry::item("&Medium", "Ctrl+2", Shortcut::ctrl('2')),
        Entry::item("&Full", "Ctrl+3", Shortcut::ctrl('3')),
        Entry::item("&Wide", "Ctrl+4", Shortcut::ctrl('4')),
        Entry::item("Detai&led", "Ctrl+5", Shortcut::ctrl('5')),
        Entry::item("&Descriptions", "Ctrl+6", Shortcut::ctrl('6')),
        Entry::item("Lon&g descriptions", "Ctrl+7", Shortcut::ctrl('7')),
        Entry::item("Fil&e owners", "Ctrl+8", Shortcut::ctrl('8')),
        Entry::item("File lin&ks", "Ctrl+9", Shortcut::ctrl('9')),
        Entry::item("&Alternative full", "Ctrl+0", Shortcut::ctrl('0')),
        Entry::Separator,
        Entry::item("&Info panel", "Ctrl+L", Shortcut::ctrl('l')),
        Entry::command("&Tree panel", "Ctrl+T", MenuCommand::Tree),
        Entry::command("Quick &view", "Ctrl+Q", MenuCommand::QuickView),
        Entry::Separator,
        Entry::command("&Sort modes", "Ctrl+F12", MenuCommand::Sort),
        Entry::command("Show lo&ng names", "Ctrl+N", MenuCommand::LongNames),
        Entry::command("Panel &On/Off", toggle, MenuCommand::TogglePanel(side)),
        Entry::item("&Re-read", "Ctrl+R", Shortcut::ctrl('r')),
        Entry::item("&Change drive", drive, Shortcut::alt(KeyCode::F(drive_key))),
    ]
}
/// What can be done to the files under the cursor.
///
/// Far's eighteen rows, then a group of this program's own: it has a new-file
/// key and an external editor that Far reaches differently, and a menu that
/// omitted them would be a map with roads missing.
pub(super) const FILES: [Entry; 24] = [
    Entry::item("&View", "F3", Shortcut::plain(KeyCode::F(3))),
    Entry::item("&Edit", "F4", Shortcut::plain(KeyCode::F(4))),
    Entry::item("&Copy", "F5", Shortcut::plain(KeyCode::F(5))),
    Entry::item("&Rename or move", "F6", Shortcut::plain(KeyCode::F(6))),
    Entry::command("Lin&k", "Alt+F6", MenuCommand::Link),
    Entry::item("&Make folder", "F7", Shortcut::plain(KeyCode::F(7))),
    Entry::item("&Delete", "F8", Shortcut::plain(KeyCode::F(8))),
    Entry::command("&Wipe", "Alt+Del", MenuCommand::Wipe),
    Entry::Separator,
    Entry::command("Add &to archive", "Shift+F1", MenuCommand::Archive),
    Entry::command("E&xtract files", "Shift+F2", MenuCommand::Extract),
    Entry::command(
        "Arc&hive commands",
        "Shift+F3",
        MenuCommand::ArchiveCommands,
    ),
    Entry::Separator,
    Entry::command("File &attributes", "Ctrl+A", MenuCommand::Attributes),
    Entry::command("A&pply command", "Ctrl+G", MenuCommand::ApplyCommand),
    Entry::command("Descri&be files", "Ctrl+Z", MenuCommand::Describe),
    Entry::Separator,
    Entry::item(
        "Select &group",
        "Gray +",
        Shortcut::plain(KeyCode::Char('+')),
    ),
    Entry::item(
        "U&nselect group",
        "Gray -",
        Shortcut::plain(KeyCode::Char('-')),
    ),
    Entry::item(
        "&Invert selection",
        "Gray *",
        Shortcut::plain(KeyCode::Char('*')),
    ),
    Entry::item("Re&store selection", "Ctrl+M", Shortcut::ctrl('m')),
    Entry::Separator,
    Entry::item("New &file", "Shift+F4", Shortcut::shift(KeyCode::F(4))),
    Entry::item("Edit with $EDIT&OR", "Alt+F4", Shortcut::alt(KeyCode::F(4))),
];
/// Everything that is not about one file.
pub(super) const COMMANDS: &[Entry] = &[
    Entry::command("Action statistics (stats)", "", MenuCommand::Performance),
    Entry::command("Language servers (LSP)", "", MenuCommand::LanguageServers),
    Entry::command("Update pac&kages", "", MenuCommand::UpdatePackages),
    Entry::command(
        "Edit PATH (current user)",
        "",
        MenuCommand::Path(crate::path_env::Scope::User),
    ),
    Entry::command(
        "Edit PATH (all users)",
        "",
        MenuCommand::Path(crate::path_env::Scope::System),
    ),
    Entry::command("&Find file", "Alt+F7", MenuCommand::FindFile),
    Entry::command("&History", "Alt+F8", MenuCommand::History),
    Entry::command("&Video mode", "Alt+F9", MenuCommand::VideoMode),
    Entry::command("Fi&nd folder", "Alt+F10", MenuCommand::FindFolder),
    Entry::command("File view histor&y", "Alt+F11", MenuCommand::ViewHistory),
    Entry::command("F&olders history", "Alt+F12", MenuCommand::FolderHistory),
    Entry::Separator,
    Entry::item("&Swap panels", "Ctrl+U", Shortcut::ctrl('u')),
    Entry::item("&Panels On/Off", "Ctrl+O", Shortcut::ctrl('o')),
    Entry::command("&Compare folders", "", MenuCommand::Compare),
    Entry::Separator,
    Entry::command("Edit user &menu", "", MenuCommand::UserMenu),
    Entry::command("File &associations", "", MenuCommand::Associations),
    Entry::command("Fol&der shortcuts", "", MenuCommand::Shortcuts),
    Entry::command("F&ile panel filter", "Ctrl+I", MenuCommand::Filter),
    Entry::Separator,
    Entry::item("Pl&ugin commands", "F11", Shortcut::plain(KeyCode::F(11))),
    Entry::command("Install MCP s&erver", "", MenuCommand::McpInstall),
    Entry::command("Sc&reens list", "F12", MenuCommand::Screens),
    Entry::command("Task &list", "Ctrl+W", MenuCommand::Tasks),
    Entry::command("Ho&tplug devices list", "", MenuCommand::Devices),
    Entry::Separator,
    Entry::item("&Go to root", "Ctrl+\\", Shortcut::ctrl('\\')),
    #[cfg(windows)]
    Entry::item(
        "&WSL manager",
        "Ctrl+Alt+W",
        Shortcut::new(
            KeyCode::Char('w'),
            KeyModifiers::CONTROL.union(KeyModifiers::ALT),
        ),
    ),
];
