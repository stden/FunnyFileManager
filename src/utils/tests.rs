//! Unit tests and shared test helpers for `utils`.

use super::*;

use std::sync::{Mutex, OnceLock};

mod copy_dir_all;
mod copy_dir_all_trees;
mod copy_failures;
mod format_size;
mod refresh_panels;
mod refresh_panels_deleted_folders;
#[cfg(unix)]
mod rewrite_file;
mod unique_destination_counter;
mod unique_destination_names;
mod unnamed_file;

/// Serialise all tests that read/write the `FORCE_COPY_DIR_ALL_STEP` global.
pub fn copy_step_mutex() -> &'static Mutex<()> {
    static COPY_STEP_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    COPY_STEP_LOCK.get_or_init(|| Mutex::new(()))
}
