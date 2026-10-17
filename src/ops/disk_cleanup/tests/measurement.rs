//! Purpose: Cleanup size and availability edge cases without touching host caches.

use super::*;
use crate::models::{CleanupCandidate, CleanupKind};

#[test]
fn zero_byte_heap_dumps_remain_available_for_cleanup() {
    let temp = TempDir::new().unwrap();
    let dump = temp.path().join("empty.hprof");
    fs::write(&dump, b"").unwrap();
    let mut dialog = dialog(vec![heap_dumps(temp.path())]);

    super::super::measure(&mut dialog, temp.path());

    assert!(dialog.candidates[0].exists());
    assert!(dialog.candidates[0].is_checked());
    assert_eq!(dialog.candidates[0].size(), Some(FileSize::from_bytes(0)));
}

#[test]
fn remeasuring_a_removed_cache_clears_its_size_and_selection() {
    let temp = TempDir::new().unwrap();
    let cache = temp.path().join("Temp");
    fs::create_dir(&cache).unwrap();
    fs::write(cache.join("old.bin"), b"old").unwrap();
    let temp_row = CleanupCandidate {
        kind: CleanupKind::Contents,
        ..candidate("Temp files", &cache)
    };
    let mut dialog = dialog(vec![temp_row]);
    super::super::measure(&mut dialog, temp.path());
    assert_eq!(dialog.selected_bytes(), 3);
    fs::remove_dir_all(cache).unwrap();

    super::super::measure(&mut dialog, temp.path());

    assert!(!dialog.candidates[0].exists());
    assert!(!dialog.candidates[0].is_checked());
    assert_eq!(dialog.candidates[0].size(), None);
    assert!(dialog.selected_paths().is_empty());
}

#[test]
fn an_unreadable_heap_dump_location_is_unavailable() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("not-a-directory");
    fs::write(&file, b"data").unwrap();
    let mut dialog = dialog(vec![heap_dumps(&file)]);

    super::super::measure(&mut dialog, temp.path());

    assert!(!dialog.candidates[0].exists());
    assert!(!dialog.candidates[0].is_checked());
    assert_eq!(dialog.candidates[0].size(), None);
}

#[test]
fn heap_dump_measurement_ignores_directories_and_nested_dumps() {
    let temp = TempDir::new().unwrap();
    fs::create_dir(temp.path().join("folder.hprof")).unwrap();
    fs::write(temp.path().join("folder.hprof/nested.hprof"), b"nested").unwrap();
    fs::write(temp.path().join("keep.txt"), b"unrelated").unwrap();
    let mut dialog = dialog(vec![heap_dumps(temp.path())]);

    super::super::measure(&mut dialog, temp.path());

    assert!(!dialog.candidates[0].exists());
    assert_eq!(dialog.candidates[0].size(), None);
}

#[test]
fn opening_resolves_relative_candidates_against_the_active_panel() {
    let temp = TempDir::new().unwrap();
    fs::create_dir(temp.path().join("target")).unwrap();
    fs::write(temp.path().join("target/build"), b"build").unwrap();
    let mut app = app(temp.path(), vec![]);
    let dialog = dialog(vec![candidate("Cargo", std::path::Path::new("target"))]);

    super::super::open_dialog(&mut app, dialog);

    let Overlay::DiskCleanup(dialog) = &app.overlay else {
        panic!("cleanup")
    };
    assert_eq!(dialog.candidates[0].path, temp.path().join("target"));
    assert_eq!(dialog.candidates[0].size(), Some(FileSize::from_bytes(5)));
    assert!(dialog.candidates[0].exists());
    assert!(!dialog.candidates[0].is_checked());
}
