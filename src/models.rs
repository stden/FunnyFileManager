//! Purpose: Defines core app state, domain enums, and panel data models.
//!
// The test children reach these through `use super::*`.
#[cfg(test)]
use std::{
    io,
    path::{Path, PathBuf},
};

mod app;
mod app_overlay;
pub mod apply;
mod archive;
mod associations;
mod attributes;
mod bookmarks;
mod clipboard;
mod command_line;
mod describe;
mod disk_cleanup;
mod drive_entry;
mod drives;
mod editor;
pub(crate) use editor::lsp;
pub use editor::{EditorLspState, LspFeature, ServerState};
mod entry;
mod file_name;
mod filter;
mod findfile;
mod histlist;
mod histories;
mod hotplug;
mod overlay;
mod panel;
mod panelview;
mod quickview;
mod screens;
mod settings_dialog;
mod shortcuts;
mod shortname;
mod sortmenu;
mod tasks;
mod tree;
mod usermenu;
#[cfg(feature = "keepass")]
mod vault_browser;
mod viewer;
mod viewport;
mod wsl;

pub use app::App;
pub use apply::ApplyPrompt;
pub use archive::{
    ArchiveCommand, ArchiveField, ArchiveFormat, ArchiveMenu, ArchivePrompt, ExtractField,
    ExtractPrompt, ARCHIVE_SUFFIXES,
};
pub use associations::AssociationList;
pub use attributes::{
    format_time, AttributesDialog, AttributesField, Check, EntrySnapshot, Flag, FlagBox, FlagGroup,
    InvalidTime, RequestedTimes,
};
pub use bookmarks::{BookmarkSlot, Bookmarks, MAX_BOOKMARKS};
pub use clipboard::ClipboardSlot;
pub use command_line::{Candidate, CandidateKind, CommandLine, Menu, Recall};
pub use describe::DescribePrompt;
pub use disk_cleanup::{CleanupCandidate, CleanupField, CleanupKind, DiskCleanupDialog};
#[cfg(all(windows, test))]
use drive_entry::windows_drive_paths_from_mask;
pub use drive_entry::DriveEntry;
pub use drives::{find_by_key as drive_by_key, rows as drive_rows, DriveRow, RowKind};
pub use editor::{
    CharColumn, EditState, Editor, EditorMotion, EditorSelection, ExitChoice, FileFormat,
    LineEnding, SelectionMode,
};
pub use entry::{Attributes, DirEntry, EntryKind, FileSize, Modified};
pub use file_name::FileName;
pub use filter::{FilterPrompt, PanelFilter};
pub use findfile::{FindFile, FindFileField};
pub use histlist::HistoryList;
pub use histories::{FolderHistory, PathHistory, ViewHistory};
pub use hotplug::DeviceList;
pub use overlay::{
    Confirm, LinkType, MaskPrompt, MaskPurpose, NameField, NamePrompt, NamePurpose, Overlay,
};
pub(crate) use panel::{DirectoryListing, PanelMetadata};
pub use panel::{Panel, PanelDetails, PanelOptions, PanelView};
pub use panelview::{PanelKind, PanelsShown, SortMode};
pub use quickview::{Preview, QuickView, Refusal, TextPreview};
pub use screens::ScreenList;
pub use settings_dialog::SettingsDialog;
pub use shortcuts::ShortcutList;
pub use shortname::{ShortName, ShortNames};
pub use sortmenu::{SortMenu, SortOrder, SortRow};
pub use tasks::TaskList;
pub use tree::{Branch, Expansion, FolderTree, Guide, TreeRow};
pub use usermenu::UserMenu;

mod menu_dialog;
pub use menu_dialog::{ChoiceTarget, MenuChoice, MenuDialog};
pub mod settings;
#[cfg(feature = "keepass")]
pub use vault_browser::KdbxOverlay;
pub use viewer::{Encoding, ViewSource, ViewedFile, Viewer};
pub use viewport::{Viewport, FALLBACK_TERMINAL_HEIGHT};
pub use wsl::{
    DistributionName, WslActivity, WslDialog, WslOperation, WslOutput, WslRequest, WslRequestId,
    WslRequests,
};

/// What fills the frame behind any modal overlay.
///
/// Split out of [`Overlay`] because hiding the panels is not a dialog: this
/// says *what is on screen*, while `Overlay` says *which dialog is open*. As
/// one enum the two were mutually exclusive, so the command line could not be
/// used while the console showed — and letting you keep working with the
/// output in front of you is the entire point of Far Manager's `Ctrl+O`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ViewMode {
    /// The dual-panel view.
    #[default]
    Panels,
    /// `Ctrl+O`: the panels are lifted off and the console transcript shows.
    Console,
}

impl ViewMode {
    /// The other mode — what `Ctrl+O` switches to.
    #[must_use]
    pub const fn toggled(self) -> Self {
        match self {
            Self::Panels => Self::Console,
            Self::Console => Self::Panels,
        }
    }
}

/// Whether the event loop should keep running.
///
/// One representation, replacing `should_quit: bool` *beside* a `Flow`
/// enum returned by the key dispatcher. The loop condition read the flag while
/// the dispatch site matched the enum, so "are we leaving" had two answers
/// that had to be kept in step by hand — and the overlay layer could only
/// reach one of them, which is why the flag existed at all.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Lifecycle {
    #[default]
    Running,
    /// Something asked to leave: F10, Ctrl+C, or the `quit` / `exit` command.
    Quitting,
}

impl Lifecycle {
    #[must_use]
    pub const fn is_running(self) -> bool {
        matches!(self, Self::Running)
    }
}

/// Which panel is currently active: left or right
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PanelSide {
    Left,
    Right,
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_name_prompt;
#[cfg(test)]
mod tests_overlay_accessors;
#[cfg(test)]
mod tests_panel_edge_cases;
#[cfg(test)]
mod tests_panel_enter_dotdot;
#[cfg(test)]
mod tests_panel_enter_marks;
#[cfg(test)]
mod tests_panel_far_sort;
#[cfg(test)]
mod tests_panel_navigation;
#[cfg(test)]
mod tests_panel_navigation_invariant;
#[cfg(test)]
mod tests_panel_paging;
#[cfg(test)]
mod tests_panel_sorting;
#[cfg(test)]
mod tests_panel_symlinks;
#[cfg(test)]
mod tests_settings;
