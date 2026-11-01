use funny_file_manager::models::FileName;
use funny_file_manager::models::FileSize;
/// Additional tests to improve coverage for `FunnyFileManager`.
///
/// Covers: marked file cleanup after reload, `DriveEntry` display, Panel edge cases,
/// handler error paths, command execution, and UI rendering edge cases.
use funny_file_manager::models::ViewMode;
use funny_file_manager::models::{App, DriveEntry, Overlay, Panel, PanelSide, SortMode, Viewer};
use funny_file_manager::ui::{centered_rect, ui};
use funny_file_manager::utils::{copy_dir_all, unique_destination};
use ratatui::backend::TestBackend;
use ratatui::style::Color;
use ratatui::Terminal;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[path = "additional_tests/app_panels.rs"]
mod app_panels;
#[path = "additional_tests/find_and_targets.rs"]
mod find_and_targets;
#[path = "additional_tests/marks_and_reload.rs"]
mod marks_and_reload;
#[path = "additional_tests/panel_navigation.rs"]
mod panel_navigation;
#[path = "additional_tests/rendering.rs"]
mod rendering;
#[path = "additional_tests/sorting.rs"]
mod sorting;
#[path = "additional_tests/utils.rs"]
mod utils;
