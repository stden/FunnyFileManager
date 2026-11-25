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

mod app_panels;
mod find_matching;
mod panel_selection;
mod value_types;
mod virtual_path;
