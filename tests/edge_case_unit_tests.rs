//! Edge-case unit tests for `FunnyFileManager`
//! Targets uncovered branches and edge cases across models, utils, s3, and handlers modules.

use funny_file_manager::models::FileName;
use funny_file_manager::models::FileSize;
use funny_file_manager::models::{App, DriveEntry, Panel, PanelSide, SortMode, Viewer};
#[cfg(feature = "s3")]
use funny_file_manager::s3::VirtualPath;
use funny_file_manager::scroll::ScrollTop;
use std::fs::{self, File};
use std::io;
use std::path::PathBuf;
use tempfile::TempDir;

#[path = "edge_case_unit_tests/app_panels.rs"]
mod app_panels;
#[path = "edge_case_unit_tests/find_matching.rs"]
mod find_matching;
#[path = "edge_case_unit_tests/panel_selection.rs"]
mod panel_selection;
#[path = "edge_case_unit_tests/value_types.rs"]
mod value_types;
#[path = "edge_case_unit_tests/virtual_path.rs"]
mod virtual_path;
