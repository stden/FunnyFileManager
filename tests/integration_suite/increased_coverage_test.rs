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

mod app_and_drives;
mod find_and_sort;
mod panel_navigation;
mod panel_selection;
mod s3;
