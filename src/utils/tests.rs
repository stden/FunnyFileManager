//! Unit tests and shared test helpers for `utils`.
#![allow(unused_imports)]

use super::*;
use crate::models::Bookmarks;
use crate::models::FileSize;
use crate::quicksearch::QuickSearch;
use crate::status::Status;

use crate::models::{App, Panel, PanelSide};

use std::ffi::OsStr;

use std::fs::File;

use std::io::Write;

use std::sync::{Mutex, OnceLock};

use tempfile::TempDir;

/// Serialise all tests that read/write the `FORCE_COPY_DIR_ALL_STEP` global.
pub fn copy_step_mutex() -> &'static Mutex<()> {
    static COPY_STEP_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    COPY_STEP_LOCK.get_or_init(|| Mutex::new(()))
}

#[test]
fn test_format_size_bytes() {
    assert_eq!(FileSize::from_bytes(0).to_string(), "0 B");
    assert_eq!(FileSize::from_bytes(1).to_string(), "1 B");
    assert_eq!(FileSize::from_bytes(999).to_string(), "999 B");
    assert_eq!(FileSize::from_bytes(1023).to_string(), "1023 B");
}

#[test]
fn test_format_size_kilobytes() {
    assert_eq!(FileSize::from_bytes(1024).to_string(), "1.00 KB");
    assert_eq!(FileSize::from_bytes(1536).to_string(), "1.50 KB");
    assert_eq!(FileSize::from_bytes(2048).to_string(), "2.00 KB");
    assert_eq!(FileSize::from_bytes(1_048_575).to_string(), "1024.00 KB");
}

#[test]
fn test_format_size_megabytes() {
    assert_eq!(FileSize::from_bytes(1_048_576).to_string(), "1.00 MB");
    assert_eq!(FileSize::from_bytes(1_572_864).to_string(), "1.50 MB");
    assert_eq!(FileSize::from_bytes(2_097_152).to_string(), "2.00 MB");
    assert_eq!(
        FileSize::from_bytes(1_073_741_823).to_string(),
        "1024.00 MB"
    );
}

#[test]
fn test_format_size_gigabytes() {
    assert_eq!(FileSize::from_bytes(1_073_741_824).to_string(), "1.00 GB");
    assert_eq!(FileSize::from_bytes(1_610_612_736).to_string(), "1.50 GB");
    assert_eq!(FileSize::from_bytes(2_147_483_648).to_string(), "2.00 GB");
    assert_eq!(FileSize::from_bytes(10_737_418_240).to_string(), "10.00 GB");
}

#[test]
fn test_unique_destination_no_conflict() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();
    let file_name = std::ffi::OsStr::new("test.txt");

    let dest = unique_destination(base, file_name).unwrap();
    assert_eq!(dest, base.join("test.txt"));
}

#[test]
fn test_unique_destination_with_conflicts() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    // Create existing files
    File::create(base.join("test.txt")).unwrap();
    File::create(base.join("test_1.txt")).unwrap();

    let file_name = std::ffi::OsStr::new("test.txt");
    let dest = unique_destination(base, file_name).unwrap();
    assert_eq!(dest, base.join("test_2.txt"));
}

#[test]
fn test_unique_destination_no_extension() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    File::create(base.join("test")).unwrap();

    let file_name = std::ffi::OsStr::new("test");
    let dest = unique_destination(base, file_name).unwrap();
    assert_eq!(dest, base.join("test_1"));
}

#[test]
fn test_unique_destination_multiple_dots() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    File::create(base.join("test.backup.txt")).unwrap();

    let file_name = std::ffi::OsStr::new("test.backup.txt");
    let dest = unique_destination(base, file_name).unwrap();
    assert_eq!(dest, base.join("test.backup_1.txt"));
}

#[test]
fn test_unique_destination_empty_extension() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    File::create(base.join("test.")).unwrap();

    let file_name = std::ffi::OsStr::new("test.");
    let dest = unique_destination(base, file_name).unwrap();
    // When extension is empty, it's treated as no extension
    assert_eq!(dest, base.join("test_1"));
}

#[test]
fn test_unique_destination_dotfile() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    File::create(base.join(".env")).unwrap();

    let file_name = std::ffi::OsStr::new(".env");
    let dest = unique_destination(base, file_name).unwrap();
    assert_eq!(dest, base.join(".env_1"));
}

#[test]
fn test_unique_destination_unicode_name() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    File::create(base.join("данные.txt")).unwrap();

    let dest = unique_destination(base, OsStr::new("данные.txt")).unwrap();
    assert_eq!(dest, base.join("данные_1.txt"));
}

#[test]
fn test_unique_destination_hidden_file_with_extension() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    File::create(base.join(".config.json")).unwrap();

    let dest = unique_destination(base, OsStr::new(".config.json")).unwrap();
    assert_eq!(dest, base.join(".config_1.json"));
}

#[test]
fn test_unique_destination_sparse_conflict_numbers() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    File::create(base.join("report.txt")).unwrap();
    File::create(base.join("report_1.txt")).unwrap();
    File::create(base.join("report_3.txt")).unwrap();

    let dest = unique_destination(base, OsStr::new("report.txt")).unwrap();
    assert_eq!(dest, base.join("report_2.txt"));
}

#[test]
fn test_unique_destination_preserves_base_directory() {
    let temp_dir = TempDir::new().unwrap();
    let nested = temp_dir.path().join("nested");
    fs::create_dir(&nested).unwrap();
    File::create(nested.join("image.png")).unwrap();

    let dest = unique_destination(&nested, OsStr::new("image.png")).unwrap();
    assert!(dest.starts_with(&nested));
    assert_eq!(dest, nested.join("image_1.png"));
}

#[test]
fn test_copy_dir_all_mutation_killer() -> io::Result<()> {
    let _lock = copy_step_mutex().lock().expect("copy step mutex");
    reset_forced_errors();

    let temp_dir = TempDir::new().expect("test setup");
    let src = temp_dir.path().join("src");
    let dst = temp_dir.path().join("dst");

    fs::create_dir(&src).expect("test setup");
    File::create(src.join("file.txt"))
        .expect("test setup")
        .write_all(b"content")
        .expect("test setup");

    // Test that function doesn't just return Ok(()) without doing anything
    copy_dir_all(&src, &dst).expect("test setup");

    // Verify the copy actually happened
    assert!(dst.exists());
    assert!(dst.join("file.txt").exists());
    assert_eq!(
        fs::read_to_string(dst.join("file.txt")).expect("test setup"),
        "content"
    );

    Ok(())
}

#[test]
fn test_copy_dir_all_src_not_exist() -> io::Result<()> {
    let _lock = copy_step_mutex().lock().expect("copy step mutex");
    let temp_dir = TempDir::new().expect("test setup");
    let src = temp_dir.path().join("non_existent_src");
    let dst = temp_dir.path().join("dst");

    let result = copy_dir_all(&src, &dst);
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_copy_dir_all_src_not_exist_rebase_copy() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let src = temp_dir.path().join("non_existent_src");
    let dst = temp_dir.path().join("dst");

    let result = copy_dir_all(&src, &dst);
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_refresh_panels_mutation_killer() -> io::Result<()> {
    let _lock = super::refresh_panels_mutex().lock().expect("refresh mutex");
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel.path = temp_dir.path().to_path_buf();
    app.right_panel.path = temp_dir.path().to_path_buf();

    // Create initial state
    app.left_panel.load_entries().expect("test setup");
    app.right_panel.load_entries().expect("test setup");
    let initial_count = app.left_panel.entries.len();

    // Add a new file
    File::create(temp_dir.path().join("new_file.txt")).expect("test setup");

    // Test that refresh_panels actually does something
    refresh_panels(&mut app).expect("test setup");

    // Verify panels were actually refreshed
    assert!(app.left_panel.entries.len() > initial_count);
    assert!(app.right_panel.entries.len() > initial_count);
    assert!(app
        .left_panel
        .entries
        .iter()
        .any(|e| e.name == "new_file.txt"));

    Ok(())
}

#[test]
fn test_copy_dir_all_simple() -> io::Result<()> {
    let _lock = copy_step_mutex().lock().expect("copy step mutex");
    let temp_dir = TempDir::new().expect("test setup");
    let src = temp_dir.path().join("src");
    let dst = temp_dir.path().join("dst");

    fs::create_dir(&src).expect("test setup");
    File::create(src.join("file1.txt"))
        .expect("test setup")
        .write_all(b"content1")
        .expect("test setup");
    File::create(src.join("file2.txt"))
        .expect("test setup")
        .write_all(b"content2")
        .expect("test setup");

    copy_dir_all(&src, &dst).expect("test setup");

    assert!(dst.exists());
    assert!(dst.join("file1.txt").exists());
    assert!(dst.join("file2.txt").exists());

    let content1 = fs::read_to_string(dst.join("file1.txt")).expect("test setup");
    assert_eq!(content1, "content1");

    Ok(())
}

#[test]
fn test_copy_dir_all_nested() -> io::Result<()> {
    let _lock = copy_step_mutex().lock().expect("copy step mutex");
    let temp_dir = TempDir::new().expect("test setup");
    let src = temp_dir.path().join("src");
    let dst = temp_dir.path().join("dst");

    fs::create_dir(&src).expect("test setup");
    fs::create_dir(src.join("subdir1")).expect("test setup");
    fs::create_dir(src.join("subdir1").join("subsubdir")).expect("test setup");
    fs::create_dir(src.join("subdir2")).expect("test setup");

    File::create(src.join("root.txt"))
        .expect("test setup")
        .write_all(b"root")
        .expect("test setup");
    File::create(src.join("subdir1").join("file1.txt"))
        .expect("test setup")
        .write_all(b"sub1")
        .expect("test setup");
    File::create(src.join("subdir1").join("subsubdir").join("deep.txt"))
        .expect("test setup")
        .write_all(b"deep")
        .expect("test setup");
    File::create(src.join("subdir2").join("file2.txt"))
        .expect("test setup")
        .write_all(b"sub2")
        .expect("test setup");

    copy_dir_all(&src, &dst).expect("test setup");

    assert!(dst.exists());
    assert!(dst.join("root.txt").exists());
    assert!(dst.join("subdir1").join("file1.txt").exists());
    assert!(dst
        .join("subdir1")
        .join("subsubdir")
        .join("deep.txt")
        .exists());
    assert!(dst.join("subdir2").join("file2.txt").exists());

    Ok(())
}

#[test]
fn test_copy_dir_all_empty_dir() -> io::Result<()> {
    let _lock = copy_step_mutex().lock().expect("copy step mutex");
    let temp_dir = TempDir::new().expect("test setup");
    let src = temp_dir.path().join("src");
    let dst = temp_dir.path().join("dst");

    fs::create_dir(&src).expect("test setup");

    copy_dir_all(&src, &dst).expect("test setup");

    assert!(dst.exists());
    assert!(dst.is_dir());
    assert_eq!(fs::read_dir(&dst).expect("test setup").count(), 0);

    Ok(())
}

#[test]
fn test_copy_dir_all_preserves_existing_destination_files() -> io::Result<()> {
    let _lock = copy_step_mutex().lock().expect("copy step mutex");
    reset_forced_errors();

    let temp_dir = TempDir::new().expect("test setup");
    let src = temp_dir.path().join("src");
    let dst = temp_dir.path().join("dst");

    fs::create_dir(&src).expect("test setup");
    fs::create_dir(&dst).expect("test setup");
    File::create(src.join("new.txt"))?.write_all(b"new-data")?;
    File::create(dst.join("existing.txt"))?.write_all(b"keep-me")?;

    copy_dir_all(&src, &dst)?;

    assert_eq!(fs::read_to_string(dst.join("existing.txt"))?, "keep-me");
    assert_eq!(fs::read_to_string(dst.join("new.txt"))?, "new-data");
    Ok(())
}

#[test]
fn test_copy_dir_all_copies_zero_length_file() -> io::Result<()> {
    let _lock = copy_step_mutex().lock().expect("copy step mutex");
    reset_forced_errors();

    let temp_dir = TempDir::new().expect("test setup");
    let src = temp_dir.path().join("src");
    let dst = temp_dir.path().join("dst");

    fs::create_dir(&src).expect("test setup");
    File::create(src.join("empty.txt"))?;

    copy_dir_all(&src, &dst)?;

    let metadata = fs::metadata(dst.join("empty.txt"))?;
    assert!(metadata.is_file());
    assert_eq!(metadata.len(), 0);
    Ok(())
}

#[test]
fn test_refresh_panels() -> io::Result<()> {
    let _lock = super::refresh_panels_mutex().lock().expect("refresh mutex");
    let temp_dir = TempDir::new().expect("test setup");

    // Create initial files
    File::create(temp_dir.path().join("initial.txt")).expect("test setup");

    let mut app = App {
        left_panel: Panel::new(temp_dir.path().to_path_buf()).expect("test setup"),
        right_panel: Panel::new(temp_dir.path().to_path_buf()).expect("test setup"),
        active_panel: PanelSide::Left,
        status: Status::Cleared,
        overlay: crate::models::Overlay::None,
        bookmarks: Bookmarks::default(),
        lifecycle: crate::models::Lifecycle::default(),
        quick_search: QuickSearch::default(),
        view: crate::models::ViewMode::default(),
        history: crate::history::History::default(),
        console: crate::console::ConsoleLog::default(),
    };

    // Add new files after app creation
    File::create(temp_dir.path().join("new_file.txt")).expect("test setup");
    fs::create_dir(temp_dir.path().join("new_dir")).expect("test setup");

    // Refresh panels
    refresh_panels(&mut app).expect("test setup");

    // Check both panels contain the new entries
    let left_has_new_file = app
        .left_panel
        .entries
        .iter()
        .any(|e| e.name == "new_file.txt");
    let left_has_new_dir = app.left_panel.entries.iter().any(|e| e.name == "new_dir");
    let right_has_new_file = app
        .right_panel
        .entries
        .iter()
        .any(|e| e.name == "new_file.txt");
    let right_has_new_dir = app.right_panel.entries.iter().any(|e| e.name == "new_dir");

    assert!(left_has_new_file);
    assert!(left_has_new_dir);
    assert!(right_has_new_file);
    assert!(right_has_new_dir);

    Ok(())
}
