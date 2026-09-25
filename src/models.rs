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
mod lifecycle;
pub mod lsp_editor;
mod mcp_install;
pub use mcp_install::{ClientRow, Mark, McpInstallDialog, McpInstallField, McpInstallProgress};
mod overlay;
mod panel;
mod panelview;
mod plugin_commands;
mod prompts;
mod quickview;
pub mod remote_panel;
mod remote_path;
mod row_cursor;
mod screens;
mod session;
mod session_edit;
pub mod sessions_panel;
mod settings_dialog;
mod shortcuts;
mod shortname;
mod sortmenu;
mod tasks;
mod tree;
mod usermenu;
#[cfg(feature = "keepass")]
mod vault_browser;
mod view_mode;
mod viewer;
mod viewport;
mod wsl;

pub use app::{App, LiveInput, ProgramPause};
pub use apply::ApplyPrompt;
pub use archive::{
    ArchiveCommand, ArchiveField, ArchiveFormat, ArchiveMenu, ArchivePrompt, ExtractField,
    ExtractPrompt, ARCHIVE_SUFFIXES,
};
pub use associations::AssociationList;
pub use attributes::{
    format_time, AttributesDialog, AttributesField, Check, EntrySnapshot, Flag, FlagBox, FlagGroup,
    InvalidTime, RequestedTimes, TimeField,
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
    AfterSave, CharColumn, EditState, Editor, EditorMotion, EditorSelection, ExitChoice,
    FileFormat, Fingerprint, Indent, LineEnding, OverwritePrompt, SelectionMode, TabSize,
};
pub(crate) use entry::divide_rounded;
pub use entry::{Attributes, DirEntry, EntryKind, FileSize, Modified};
pub use file_name::FileName;
pub use filter::{FilterPrompt, PanelFilter};
pub use findfile::{FindFile, FindFileField};
pub use histlist::HistoryList;
pub use histories::{FolderHistory, HistoryFile, PathHistory, ViewHistory, Viewed, Visited};
pub use hotplug::DeviceList;
pub use lifecycle::Lifecycle;
pub use lsp_editor::LspEditor;
pub use overlay::{
    Confirm, LinkType, MaskPrompt, MaskPurpose, NameField, NamePrompt, NamePurpose, Overlay,
};
pub(crate) use panel::{DirectoryListing, PanelMetadata};
pub use panel::{Panel, PanelDetails, PanelOptions, PanelView};
pub use panelview::{PanelKind, PanelsShown, SortMode};
pub use plugin_commands::{PluginCommand, PluginCommands};
pub use prompts::{InputPrompt, InputPurpose, Question, QuestionPurpose};
pub use quickview::{Preview, QuickView, Refusal, TextPreview};
pub use remote_panel::RemotePanel;
pub use remote_path::RemotePath;
pub use row_cursor::{RowCursor, RowMotion};
pub use screens::ScreenList;
pub use session::{
    Auth, AuthKind, Field, HostName, InvalidField, KeyFile, Origin, Port, Problem, Protocol,
    Session, SessionName, UserName,
};
pub use session_edit::{EditPurpose, SessionEdit, SessionField};
pub use sessions_panel::{SessionRow, SessionsPanel};
pub use settings_dialog::SettingsDialog;
pub use shortcuts::ShortcutList;
pub use shortname::{ShortName, ShortNames};
pub use sortmenu::{SortMenu, SortOrder, SortRow};
pub use tasks::{
    build_view, Arrangement, CommandView, FilterTerms, KernelThreads, Sibling, Sort, SortDirection,
    SortKey, TreeLine, UserScope, ViewOptions, ViewRow,
};
pub use tasks::{
    Arrival, Attempt, Cadence, ConfirmButton, ControlRefusal, Cursor, Failure, FormFocus, Found,
    FrozenTargets, HeaderShape, Health, KillConfirm, ListMotion, MenuCursor, Modal, NiceConfirm,
    NicePlan, Notice, Operation, PidQuery, PlannedChange, PriorityForm, Refresh, Relation, Renice,
    ReniceButton, Request, Schedule, SearchBar, SearchFrom, Severity, Skipped, Table, Target,
    TaskHostSlot, TaskMenu, TextBar, BARS_PER_ROW, OVERDUE_AFTER, REFRESH_INTERVAL, STALL_LIMIT,
    WARM_UP,
};
pub use tasks::{
    Baselines, ControlError, CoreGroup, CoreSample, CpuBar, CpuCounters, CpuMeter, CpuMeters,
    CpuUsage, KernelPriority, KillOffer, Load, LoadAverage, MemoryBytes, MemoryMeter, MemorySample,
    MemoryUsage, Meters, NiceOutOfRange, NiceStep, Niceness, Owner, Percent, Pid, Priority,
    PriorityChange, PriorityClass, ProcessHost, ProcessKey, ProcessKind, ProcessReader, ProcessRow,
    ProcessSample, ProcessState, Sample, SampleError, SettableClass, Signal, SignalEffect,
    Snapshot, Start, StartTime, StateLetter, SwapMeter, SwapSample, SystemPart, SystemSample,
    TaskCounts, TaskList, TaskSession,
};
pub use tree::{Branch, Expansion, FolderTree, Guide, TreeRow};
pub use usermenu::UserMenu;

mod menu_dialog;
pub use menu_dialog::{ChoiceTarget, MenuChoice, MenuDialog};
pub mod settings;
#[cfg(feature = "keepass")]
pub use vault_browser::KdbxOverlay;
pub use view_mode::ViewMode;
pub use viewer::{ViewSource, ViewedFile, Viewer};
pub use viewport::{Viewport, FALLBACK_TERMINAL_HEIGHT};
pub use wsl::{
    DistributionName, WslActivity, WslDialog, WslOperation, WslOutput, WslRequest, WslRequestId,
    WslRequests,
};

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
