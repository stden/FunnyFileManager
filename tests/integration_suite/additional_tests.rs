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

mod app_panels;
mod find_and_targets;
mod marks_and_reload;
mod panel_navigation;
mod rendering;
mod sorting;
mod utils;
