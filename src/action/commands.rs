//! Purpose: the typed sub-commands an `Action` carries, one enum per menu or workflow.

use crate::models::{PanelSide, PanelView, SortMode};

use super::Action;

/// Commands belonging to the navigate workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Navigation {
    /// `Down` — move the selection down.
    NavDown,
    /// `Up` — move the selection up.
    NavUp,
    /// `Enter` — open the selected directory / file.
    Open,
    /// `Tab` — switch the active panel.
    SwitchPanel,
    /// `Home` — jump to the first entry.
    First,
    /// `End` — jump to the last entry.
    Last,
    /// `PgUp` — move the selection up one screen.
    PageUp,
    /// `PgDn` — move the selection down one screen.
    PageDown,
    /// `Ctrl+\` — jump to the filesystem root.
    GoRoot,
    /// `Ctrl+U` — swap the contents of the two panels.
    SwapPanels,
}

#[expect(
    non_upper_case_globals,
    reason = "established Action names kept in variant case for keymaps and callers"
)]
impl Action {
    pub const NavDown: Self = Self::Navigate(Navigation::NavDown);
    pub const NavUp: Self = Self::Navigate(Navigation::NavUp);
    pub const Open: Self = Self::Navigate(Navigation::Open);
    pub const SwitchPanel: Self = Self::Navigate(Navigation::SwitchPanel);
    pub const First: Self = Self::Navigate(Navigation::First);
    pub const Last: Self = Self::Navigate(Navigation::Last);
    pub const PageUp: Self = Self::Navigate(Navigation::PageUp);
    pub const PageDown: Self = Self::Navigate(Navigation::PageDown);
    pub const GoRoot: Self = Self::Navigate(Navigation::GoRoot);
    pub const SwapPanels: Self = Self::Navigate(Navigation::SwapPanels);
}

/// Commands belonging to the selection workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionCommand {
    /// `Insert` / `Space` — toggle the mark on the selection and advance.
    ToggleMark,
    /// `Gray +` — add every file matching a mask to the selection.
    SelectGroup,
    /// `Gray −` — take every file matching a mask out of it.
    UnselectGroup,
    /// `Gray ∗` — selected becomes unselected and the other way round.
    InvertSelection,
    /// `Ctrl+M` — put back the selection as it was before the last change.
    RestoreSelection,
}

#[expect(
    non_upper_case_globals,
    reason = "established Action names kept in variant case for keymaps and callers"
)]
impl Action {
    pub const ToggleMark: Self = Self::Selection(SelectionCommand::ToggleMark);
    pub const SelectGroup: Self = Self::Selection(SelectionCommand::SelectGroup);
    pub const UnselectGroup: Self = Self::Selection(SelectionCommand::UnselectGroup);
    pub const InvertSelection: Self = Self::Selection(SelectionCommand::InvertSelection);
    pub const RestoreSelection: Self = Self::Selection(SelectionCommand::RestoreSelection);
}

/// The Left / Right menu, row for row: what a panel shows and how.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelCommand {
    /// `Ctrl+1` … `Ctrl+0` — one of Far's ten column layouts.
    View(PanelView),
    /// `Ctrl+Q` — a preview of the other panel's cursor, or back.
    QuickView,
    /// `Ctrl+F3` … — a sort mode straight from its key.
    Sort(SortMode),
    /// `Ctrl+F1` / `Ctrl+F2` — that panel on or off the screen.
    TogglePanel(PanelSide),
    /// `Ctrl+N` — long names or their 8.3 form.
    LongNames,
}

/// The Files menu rows beyond the bare F-keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileCommand {
    /// `Alt+F6` — a symbolic or hard link to the entry under the cursor.
    Link,
    /// `Alt+Del` — overwrite, then delete.
    Wipe,
    /// `Shift+F1` — the selection into an archive.
    AddToArchive,
    /// `Shift+F2` — the selected archives out into the other panel.
    Extract,
    /// `Shift+F3` — test, list, extract here.
    ArchiveCommands,
    /// `Ctrl+A` — permissions and times of the selection.
    Attributes,
    /// `Ctrl+G` — a shell command once per selected entry.
    ApplyCommand,
    /// `Ctrl+Z` — a description for the entry under the cursor.
    Describe,
}

/// The Commands menu: searches, histories, and the program's own windows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolCommand {
    /// `Alt+F7`
    FindFile,
    /// `Alt+F8` — the command history as a list.
    History,
    /// `Alt+F9` — 25 or 50 rows.
    VideoMode,
    /// `Alt+F10`
    FindFolder,
    /// `Alt+F11` — files viewed and edited.
    ViewHistory,
    /// `Alt+F12` — folders visited.
    FolderHistory,
    /// `Ctrl+I` — the file panel filter.
    Filter,
    /// `F12` — the screens this program has: panels and console.
    Screens,
    /// `Ctrl+W` — the task list: the live process viewer.
    Tasks,
}

/// Commands belonging to the dialog workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogCommand {
    /// `F10` — ask before leaving, as Far does.
    ConfirmQuit,
    /// `Shift+F4` — open the "create new file" dialog.
    NewFile,
    /// `F1` — toggle the help overlay.
    ToggleHelp,
    /// `F2` — open the available user-command menu.
    UserMenu,
    /// `Ctrl+Alt+W` — manage Windows Subsystem for Linux.
    WslManagement,
    /// `F7` — create a directory.
    Mkdir,
    /// F9: open Far's menu bar.
    Menu,
    /// `Ctrl+L` — measure the active directory and show what is in it.
    Information,
}

#[expect(
    non_upper_case_globals,
    reason = "established Action names kept in variant case for keymaps and callers"
)]
impl Action {
    pub const ConfirmQuit: Self = Self::Dialog(DialogCommand::ConfirmQuit);
    pub const NewFile: Self = Self::Dialog(DialogCommand::NewFile);
    pub const ToggleHelp: Self = Self::Dialog(DialogCommand::ToggleHelp);
    pub const UserMenu: Self = Self::Dialog(DialogCommand::UserMenu);
    pub const WslManagement: Self = Self::Dialog(DialogCommand::WslManagement);
    pub const Mkdir: Self = Self::Dialog(DialogCommand::Mkdir);
    pub const MenuBar: Self = Self::Dialog(DialogCommand::Menu);
    pub const Information: Self = Self::Dialog(DialogCommand::Information);
}
