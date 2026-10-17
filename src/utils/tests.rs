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

#[test]
fn follow_links_copy_tree_descends_into_subdirectories_for_uncreated_destination() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let src = dir.path().join("src");
    fs::create_dir_all(src.join("nested")).expect("nested");
    fs::write(src.join("nested/file.txt"), b"ok").expect("file");
    let dst = dir.path().join("nonexistent_parent/dst");
    let rules = CopyRules {
        links: LinkPolicy::Follow,
        ..CopyRules::PLAIN
    };
    copy_dir_all_with(&src, &dst, rules).expect("copy_dir_all_with follow");
    assert_eq!(fs::read(dst.join("nested/file.txt")).unwrap(), b"ok");
}
