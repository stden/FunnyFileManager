//! Purpose: Defines core app state, domain enums, and panel data models.
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
pub(crate) mod temp_cleanup;
pub use temp_cleanup::TempCleanupDialog;
mod drive_entry;
mod drives;
mod editor;
pub(crate) use editor::{lsp, RemoteDocument};
mod entry;
mod file_name;
mod filter;
mod findfile;
pub(crate) mod help;
mod histlist;
mod histories;
mod hotplug;
mod image;
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
pub(crate) mod remote_dialog;
pub mod remote_panel;
mod remote_path;
pub(crate) mod remote_work;
pub use remote_dialog::RemoteOverlay;
mod row_cursor;
mod screens;
mod session;
mod session_edit;
pub mod sessions_panel;
mod settings_dialog;
mod shortcuts;
mod shortname;
mod sortmenu;
pub mod space_analysis;
pub mod tasks;
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
pub use disk_cleanup::{
    CandidateState, CleanupCandidate, CleanupField, CleanupKind, DiskCleanupDialog,
};
#[cfg(all(windows, test))]
use drive_entry::windows_drive_paths_from_mask;
pub use drive_entry::DriveEntry;
pub use drives::{find_by_key as drive_by_key, rows as drive_rows, DriveRow, RowKind};
pub use editor::{
    AfterSave, CharColumn, CompletionTrigger, EditState, Editor, EditorLspState, EditorModal,
    EditorMotion, EditorSelection, ExitChoice, FileFormat, Fingerprint, Indent, LineEnding,
    LspFeature, OverwritePrompt, SelectionMode, ServerState, TabSize,
};
pub(crate) use entry::divide_rounded;
pub use entry::{Attributes, DirEntry, DirectorySize, EntryKind, FileSize, Modified};
pub use file_name::FileName;
pub(crate) use filter::{FilterField, FilterInput};
pub use filter::{FilterPrompt, PanelFilter};
pub use findfile::{FindFile, FindFileField};
pub use histlist::HistoryList;
pub use histories::{FolderHistory, HistoryFile, PathHistory, ViewHistory, Viewed, Visited};
pub use hotplug::DeviceList;
pub use lifecycle::Lifecycle;
pub use lsp_editor::LspEditor;
pub use overlay::{
    Clash, ClashAnswer, ClashFocus, Confirm, Denial, ElevatedRetry, EntryFacts, ExistingFile,
    LinkType, MaskPrompt, MaskPurpose, NameField, NamePrompt, NamePurpose, OperationFailed,
    Overlay,
};
pub(crate) use panel::{DirectoryListing, DirtyProbe, PanelMetadata};
pub use panel::{Panel, PanelCollection, PanelDetails, PanelOptions, PanelView};
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
    build_view, Arrangement, Arrival, Attempt, Baselines, Cadence, CommandView, ConfirmButton,
    ControlError, ControlRefusal, CoreGroup, CoreSample, CpuBar, CpuCounters, CpuMeter, CpuMeters,
    CpuUsage, Cursor, Failure, FilterTerms, FormFocus, Found, FrozenTargets, HeaderShape, Health,
    KernelPriority, KernelThreads, KillConfirm, KillOffer, ListMotion, Load, LoadAverage,
    MemoryBytes, MemoryMeter, MemorySample, MemoryUsage, MenuCursor, Meters, Modal, NetProber,
    NiceConfirm, NiceOutOfRange, NicePlan, NiceStep, Niceness, Notice, Operation, Owner, Percent,
    Pid, PidQuery, PlannedChange, Priority, PriorityChange, PriorityClass, PriorityForm,
    ProcessHost, ProcessKey, ProcessKind, ProcessReader, ProcessRow, ProcessSample, ProcessState,
    Refresh, Relation, Renice, ReniceButton, Request, Sample, SampleError, Schedule, SearchBar,
    SearchFrom, SettableClass, Severity, Sibling, Signal, SignalEffect, Skipped, Snapshot, Sort,
    SortDirection, SortKey, Start, StartTime, StateLetter, SwapMeter, SwapSample, SystemPart,
    SystemSample, Table, Target, TaskCounts, TaskHostSlot, TaskList, TaskMenu, TaskSession,
    TaskTab, TextBar, TreeLine, UserScope, ViewOptions, ViewRow, BARS_PER_ROW, OVERDUE_AFTER,
    REFRESH_INTERVAL, STALL_LIMIT, WARM_UP,
};
pub use tree::{Branch, Expansion, FolderTree, Guide, TreeRow};
pub use usermenu::UserMenu;
mod menu_dialog;
pub use menu_dialog::{ChoiceTarget, MenuChoice, MenuDialog, MenuPrompt};
pub mod settings;
pub use image::{DecodedImage, ImageState};
#[cfg(feature = "keepass")]
pub use vault_browser::KdbxOverlay;
pub use view_mode::ViewMode;
pub use viewer::{ViewSource, ViewedFile, Viewer};
pub use viewport::{Viewport, FALLBACK_TERMINAL_HEIGHT};
pub use wsl::{
    DistributionName, WslActivity, WslDialog, WslOperation, WslOutput, WslRequest, WslRequestId,
    WslRequests,
};

mod panel_side;
pub use panel_side::PanelSide;

pub(crate) mod network_drive;
pub(crate) mod token_usage;
pub use token_usage::TokenUsagePanel;
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
pub mod things;

pub use network_drive::{
    MountPoint, NetworkAddress, NetworkDrive, NetworkDriveError, NetworkProtocol,
};
