//! Purpose: The modal overlays, and the small types the dialogs are made of.
//!
//! Split out of `models.rs` when it passed the 1000-line cap
//! `tests/file_line_limit_test.rs` enforces. The boundary is not arbitrary:
//! everything here is about something that takes over the keyboard, and
//! `models.rs` keeps what is always on screen.

mod mask_prompt;
mod name_fields;
mod name_prompt;

pub use mask_prompt::{MaskPrompt, MaskPurpose};
pub use name_fields::{LinkType, NameField, NamePurpose};
pub use name_prompt::NamePrompt;

use std::path::PathBuf;

use super::{
    ApplyPrompt, ArchiveMenu, ArchivePrompt, AssociationList, AttributesDialog, CommandLine,
    DescribePrompt, DeviceList, DriveRow, Editor, ExtractPrompt, FilterPrompt, FindFile,
    FolderTree, HistoryList, InputPrompt, PanelSide, PluginCommands, Question, ScreenList,
    SessionEdit, SettingsDialog, ShortcutList, SortMenu, TaskList, UserMenu, Viewer,
};

/// Which button a two-answer dialog has focus on.
///
/// Two named answers rather than a `bool` or an index: `Confirm::No` says what
/// it means at the call site, where `false` would need the reader to remember
/// which way round the dialog was built.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Confirm {
    /// The default, and what Enter starts on.
    #[default]
    Yes,
    No,
}

impl Confirm {
    /// The other answer — what Left, Right and Tab all move to, there being
    /// only two.
    #[must_use]
    pub const fn toggled(self) -> Self {
        match self {
            Self::Yes => Self::No,
            Self::No => Self::Yes,
        }
    }

    #[must_use]
    pub const fn is_yes(self) -> bool {
        matches!(self, Self::Yes)
    }
}

/// Represents the currently active overlay / modal dialog.
///
/// This enum replaces a collection of boolean and Option fields
/// (`show_help`, `viewer`, `command_line`, `show_drives`, `delete_confirm`, etc.)
/// and enforces that only one overlay can be active at a time.
#[derive(Clone, Default)]
pub enum Overlay {
    /// No overlay is active (normal dual-panel view)
    #[default]
    None,

    /// F1 help screen
    Help,

    /// Windows Subsystem for Linux distributions and maintenance commands.
    Wsl(Box<super::WslDialog>),
    PathEditor(Box<crate::path_env::PathEditor>),

    /// F3 file viewer (also reused for MCP status via F11 / :mcp)
    Viewer(Viewer),

    /// F4 built-in text editor
    Editor(Editor),

    /// Alt+F1 / Alt+F2 drive selection
    DriveSelection {
        side: PanelSide,
        selected_index: usize,
        /// Built once, when the dialog opens. A row carries its filesystem's
        /// size, and the event loop redraws after every key: rows rebuilt in
        /// the renderer would cost a `statvfs` per drive per frame.
        rows: Vec<DriveRow>,
    },

    /// Shift+F4 "create new file"
    NewFileInput(String),

    /// Command line (pressed ':')
    CommandLine(CommandLine),

    /// F8 delete confirmation dialog
    DeleteConfirm(Vec<PathBuf>),

    /// F10 "do you want to quit?", carrying which button has the cursor.
    QuitConfirm(Confirm),

    /// A dialog asking for a name: Shift+F6 rename, F7 make folder.
    NamePrompt(NamePrompt),

    /// Ctrl+L: what the active directory contains, measured.
    Information(Box<crate::stats::DirectoryStats>),

    /// F9: the menu bar's open heading and where the cursor is in it.
    Menu(crate::menu::Menu),
    MenuDialog(Box<super::MenuDialog>),

    /// Gray + / Gray −: which files to add to or take out of the selection.
    MaskPrompt(MaskPrompt),

    /// F5: where to copy, and how. Boxed because it is much the largest
    /// variant, and an `Overlay` sits inside `App` whether a dialog is open
    /// or not.
    CopyPrompt(Box<crate::copy::CopyPrompt>),

    /// Alt+Del: which files to wipe, asked the way F8 asks.
    WipeConfirm(Vec<PathBuf>),
    /// Shift+F1: Add to archive.
    ArchivePrompt(Box<ArchivePrompt>),
    /// Shift+F2: Extract files.
    ExtractPrompt(Box<ExtractPrompt>),
    /// Shift+F3: Archive commands.
    ArchiveCommands(ArchiveMenu),
    /// Ctrl+A: File attributes.
    Attributes(Box<AttributesDialog>),
    /// Ctrl+G: Apply command.
    ApplyCommand(ApplyPrompt),
    /// Ctrl+Z: Describe files.
    Describe(DescribePrompt),
    /// Alt+F7: Find file.
    FindFile(Box<FindFile>),
    /// Alt+F8: the command history as a list.
    CommandHistory(HistoryList),
    /// Alt+F10: Find folder — the tree as a dialog.
    FindFolder(Box<FolderTree>),
    /// Alt+F11: File view history.
    ViewHistory(HistoryList),
    /// Alt+F12: Folders history.
    FolderHistory(HistoryList),
    /// F2: the user menu.
    UserMenu(Box<UserMenu>),
    /// Commands menu: File associations.
    Associations(Box<AssociationList>),
    /// Commands menu: Folder shortcuts.
    FolderShortcuts(ShortcutList),
    /// Ctrl+I: File panel filter.
    Filter(FilterPrompt),
    /// F12: Screens list.
    Screens(ScreenList),
    /// Ctrl+W: Task list.
    Tasks(Box<TaskList>),
    /// Commands menu: Hotplug devices list.
    Hotplug(Box<DeviceList>),
    /// Commands menu: Install MCP server.
    McpInstall(Box<super::McpInstallDialog>),
    /// Ctrl+F12: Sort modes.
    SortModes(SortMenu),
    /// Options menu: one settings page.
    Settings(Box<SettingsDialog>),
    /// F4 / Shift+F4 on the sessions panel: one stored session's fields.
    SessionEdit(Box<SessionEdit>),
    /// F11: Far's Plugin commands list.
    PluginCommands(PluginCommands),
    /// One line of text — a path, a name, a password — asked for by a
    /// feature that names what it is for in [`super::InputPurpose`].
    InputPrompt(Box<InputPrompt>),
    /// A yes/no question whose Yes is named in [`super::QuestionPurpose`].
    Question(Question),

    /// `:cleanup` — the disk cleanup checklist.
    DiskCleanup(Box<super::DiskCleanupDialog>),

    /// Enter on a `.kdbx` file: the unlock prompt, then the entry browser.
    /// Wrapped in [`KdbxOverlay`](crate::models::KdbxOverlay) because this
    /// enum derives `Clone` and a session's master key must not be duplicated
    /// — the wrapper's `Clone` hands back the same vault, locked. This enum
    /// must stay without `Debug` for the same reason.
    #[cfg(feature = "keepass")]
    Kdbx(crate::models::KdbxOverlay),
}

impl Overlay {
    /// Returns true if any modal overlay is currently active.
    pub const fn is_active(&self) -> bool {
        !matches!(self, Self::None)
    }

    /// Close the current overlay (return to normal view).
    pub fn close(&mut self) {
        *self = Self::None;
    }
}
