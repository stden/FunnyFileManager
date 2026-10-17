//! Purpose: Real cleanup deletion and failure accounting, confined to temporary directories.

use super::*;

#[test]
fn a_missing_heap_dump_directory_reports_an_error_and_preserves_other_files() {
    let temp = TempDir::new().unwrap();
    let keep = temp.path().join("keep");
    fs::write(&keep, b"keep").unwrap();
    let mut app = app(temp.path(), vec![heap_dumps(&temp.path().join("missing"))]);
    run(&mut app);
    assert_eq!(fs::read(keep).unwrap(), b"keep");
    assert_eq!(
        app.status_text(),
        "Freed 0 B, 1 item(s) could not be removed"
    );
}

#[test]
fn a_refresh_failure_is_reported_after_successful_cleanup() {
    let _guard = crate::utils::refresh_panels_mutex().lock().unwrap();
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("cache");
    fs::write(&file, b"data").unwrap();
    let mut app = app(temp.path(), vec![candidate("Cache", &file)]);
    crate::utils::set_refresh_panels_error(true);
    super::super::run(&mut app);
    crate::utils::set_refresh_panels_error(false);
    assert!(!file.exists());
    assert_eq!(
        app.status_text(),
        "Freed 4 B; panels could not be refreshed"
    );
}

#[test]
fn a_path_candidate_removes_the_directory_with_its_nested_contents() {
    let temp = TempDir::new().unwrap();
    let cache = temp.path().join("cache");
    fs::create_dir_all(cache.join("nested")).unwrap();
    fs::write(cache.join("stale.log"), b"abc").unwrap();
    fs::write(cache.join("nested/data.bin"), b"12345").unwrap();
    let mut app = app(temp.path(), vec![candidate("Gradle caches", &cache)]);

    run(&mut app);

    assert!(!cache.exists());
    assert_eq!(app.status.to_string(), "Freed 8 B");
    assert!(matches!(app.overlay, Overlay::None));
}

#[test]
fn heap_dump_cleanup_preserves_a_home_directory_named_target() {
    let temp = TempDir::new().unwrap();
    let home = temp.path().join("target");
    fs::create_dir(&home).unwrap();
    fs::write(home.join("java.hprof"), b"dump").unwrap();
    fs::write(home.join("notes.txt"), b"important").unwrap();
    let mut app = app(temp.path(), vec![heap_dumps(&home)]);

    run(&mut app);

    assert_eq!(fs::read(home.join("notes.txt")).unwrap(), b"important");
    assert!(!home.join("java.hprof").exists());
    assert_eq!(app.status.to_string(), "Freed 4 B");
}

#[test]
fn cleanup_reports_actual_file_sizes_and_keeps_unselected_entries() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("delete.bin");
    let keep = temp.path().join("keep.bin");
    fs::write(&file, b"1234567").unwrap();
    fs::write(&keep, b"keep").unwrap();
    let mut selected = candidate("Delete", &file);
    selected.size = Some(FileSize::from_bytes(999));
    let mut unselected = candidate("Keep", &keep);
    unselected.checked = false;
    let mut app = app(temp.path(), vec![selected, unselected]);

    run(&mut app);

    assert!(!file.exists());
    assert_eq!(fs::read(keep).unwrap(), b"keep");
    assert_eq!(app.status.to_string(), "Freed 7 B");
    assert!(app
        .left_panel
        .entries
        .iter()
        .any(|entry| entry.name == "keep.bin"));
    assert!(!app
        .right_panel
        .entries
        .iter()
        .any(|entry| entry.name == "delete.bin"));
}

#[test]
fn a_missing_selected_file_is_reported_while_other_files_are_removed() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("delete.bin");
    fs::write(&file, b"123").unwrap();
    let rows = vec![
        candidate("Missing", &temp.path().join("missing.bin")),
        candidate("Delete", &file),
    ];
    let mut app = app(temp.path(), rows);

    run(&mut app);

    assert!(!file.exists());
    assert_eq!(
        app.status.to_string(),
        "Freed 3 B, 1 item(s) could not be removed"
    );
}

#[cfg(windows)]
#[test]
fn locked_heap_dumps_report_failure_without_counting_their_bytes_as_freed() {
    use std::os::windows::fs::OpenOptionsExt;

    let temp = TempDir::new().unwrap();
    let locked = temp.path().join("locked.hprof");
    let removable = temp.path().join("removable.hprof");
    fs::write(&locked, b"locked").unwrap();
    fs::write(&removable, b"free").unwrap();
    let _handle = fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(&locked)
        .unwrap();
    let mut app = app(temp.path(), vec![heap_dumps(temp.path())]);

    run(&mut app);

    assert!(locked.exists());
    assert!(!removable.exists());
    assert_eq!(
        app.status.to_string(),
        "Freed 4 B, 1 item(s) could not be removed"
    );
}
