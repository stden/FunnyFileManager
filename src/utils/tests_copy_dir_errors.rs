//! Unit tests: copy dir errors (split out to keep files under 500 lines).
#![allow(unused_imports)]

use super::tests::*;

use super::*;

use crate::models::{App, Panel, PanelSide};

use std::ffi::OsStr;

use std::fs::File;

use std::io::Write;

use std::sync::{Mutex, OnceLock};

use tempfile::TempDir;

#[test]
fn test_copy_dir_all_forced_errors() -> io::Result<()> {
    let _lock = copy_step_mutex().lock().expect("copy step mutex");
    reset_forced_errors();

    let temp_dir = TempDir::new().expect("test setup");
    let src = temp_dir.path().join("src");
    let dst = temp_dir.path().join("dst");
    fs::create_dir(&src).expect("test setup");
    File::create(src.join("file.txt")).expect("test setup");

    // Fail create_dir_all
    reset_forced_errors();
    set_copy_dir_all_step(COPY_DIR_ALL_FAIL_CREATE);
    assert!(copy_dir_all(&src, &dst).is_err());

    // Fail read_dir entry
    reset_forced_errors();
    set_copy_dir_all_step(COPY_DIR_ALL_FAIL_ENTRY);
    assert!(copy_dir_all(&src, &dst).is_err());

    // Fail file_type
    reset_forced_errors();
    set_copy_dir_all_step(COPY_DIR_ALL_FAIL_FILE_TYPE);
    assert!(copy_dir_all(&src, &dst).is_err());

    // Fail copy_file
    reset_forced_errors();
    set_copy_dir_all_step(COPY_DIR_ALL_FAIL_COPY_FILE);
    assert!(copy_dir_all(&src, &dst).is_err());

    // Fail recurse
    reset_forced_errors();
    let recurse_dir = src.join(".ffm_force_copy_dir_error");
    fs::create_dir(&recurse_dir).expect("test setup");
    set_copy_dir_all_step(COPY_DIR_ALL_FAIL_RECURSE);
    assert!(copy_dir_all(&src, &dst).is_err());

    Ok(())
}

// Kill: replace && with || in copy_dir_all recurse guard
// Kill: replace == with != in copy_dir_all recurse guard
#[test]
fn test_copy_dir_all_recurse_only_fails_on_magic_name() -> io::Result<()> {
    let _lock = copy_step_mutex().lock().expect("copy step mutex");
    reset_forced_errors();

    let temp_dir = TempDir::new().expect("test setup");
    let src = temp_dir.path().join("src");
    let dst = temp_dir.path().join("dst");

    fs::create_dir(&src).expect("test setup");
    // Normal subdirectory (NOT the magic name)
    let normal_sub = src.join("normal_subdir");
    fs::create_dir(&normal_sub).expect("test setup");
    File::create(normal_sub.join("inner.txt"))
        .expect("test setup")
        .write_all(b"data")
        .expect("test setup");

    set_copy_dir_all_step(COPY_DIR_ALL_FAIL_RECURSE);
    // Must succeed: FAIL_RECURSE only triggers on ".ffm_force_copy_dir_error"
    copy_dir_all(&src, &dst).expect("should succeed for non-magic subdir");
    assert!(dst.join("normal_subdir").join("inner.txt").exists());
    assert_eq!(
        fs::read_to_string(dst.join("normal_subdir").join("inner.txt"))?,
        "data"
    );

    reset_forced_errors();
    Ok(())
}

#[test]
fn test_refresh_panels_forced_error() -> io::Result<()> {
    let _lock = super::refresh_panels_mutex().lock().expect("refresh mutex");
    let mut app = App::new().expect("test setup");
    set_refresh_panels_error(true);
    assert!(refresh_panels(&mut app).is_err());
    set_refresh_panels_error(false);
    Ok(())
}
