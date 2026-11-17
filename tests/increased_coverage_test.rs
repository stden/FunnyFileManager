//! Purpose: Adds targeted tests that increase coverage for models, utils, and optional S3.
//!

use funny_file_manager::models::FileName;
use funny_file_manager::models::{App, DriveEntry, Panel, PanelSide, SortMode};
#[cfg(feature = "s3")]
use funny_file_manager::s3::{list_aws_profiles, VirtualPath};
use std::fs;
use std::io;
use std::path::PathBuf;
use tempfile::TempDir;

#[path = "increased_coverage_test/app_and_drives.rs"]
mod app_and_drives;
#[path = "increased_coverage_test/find_and_sort.rs"]
mod find_and_sort;
#[path = "increased_coverage_test/panel_navigation.rs"]
mod panel_navigation;
#[path = "increased_coverage_test/panel_selection.rs"]
mod panel_selection;
#[path = "increased_coverage_test/s3.rs"]
mod s3;
