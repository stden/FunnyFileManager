//! Purpose: Covers utility and model behavior used by mutation testing.
//!

use funny_file_manager::models::App;
use funny_file_manager::models::FileSize;
use funny_file_manager::utils::{copy_dir_all, refresh_panels, unique_destination};
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::PathBuf;
use tempfile::TempDir;

mod copy_and_refresh;
mod format_size;
mod unique_destination;
