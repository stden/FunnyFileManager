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

#[path = "mutation_tests/copy_and_refresh.rs"]
mod copy_and_refresh;
#[path = "mutation_tests/format_size.rs"]
mod format_size;
#[path = "mutation_tests/unique_destination.rs"]
mod unique_destination;
