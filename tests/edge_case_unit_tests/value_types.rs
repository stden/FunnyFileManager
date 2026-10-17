//! Purpose: Model values: sort modes, drive entries, the viewer and size formatting.

use super::*;

// ---------------------------------------------------------------------------
// 1. SortMode: verify full cycle returns to start and labels are stable
// ---------------------------------------------------------------------------

#[test]
fn test_sort_mode_full_cycle_returns_to_start() {
    let start = SortMode::Name;
    let after_one = start.next();
    let after_two = after_one.next();
    let after_three = after_two.next();
    assert_eq!(start, after_three);
    assert_ne!(start, after_one);
    assert_ne!(after_one, after_two);
}

#[test]
fn test_sort_mode_labels_are_nonempty() {
    for mode in [SortMode::Name, SortMode::Extension, SortMode::Size] {
        assert!(!mode.label().is_empty());
    }
}

#[test]
fn test_sort_mode_default_is_name() {
    let mode: SortMode = SortMode::default();
    assert_eq!(mode, SortMode::Name);
    assert_eq!(mode.label(), "Name");
}

// ---------------------------------------------------------------------------
// 2. DriveEntry::display edge cases
// ---------------------------------------------------------------------------

#[test]
fn test_drive_entry_display_local_root() {
    let entry = DriveEntry::Local(PathBuf::from("/"));
    assert_eq!(entry.display(), "/");
}

#[test]
fn test_drive_entry_display_s3_profile_with_spaces() {
    let entry = DriveEntry::S3Profile("my profile".to_owned());
    assert_eq!(entry.display(), "S3: my profile");
}

#[test]
fn test_drive_entry_display_s3_profile_empty_name() {
    let entry = DriveEntry::S3Profile(String::new());
    assert_eq!(entry.display(), "S3: ");
}

// ---------------------------------------------------------------------------
// 8. Viewer default values
// ---------------------------------------------------------------------------

#[test]
fn test_viewer_default_has_empty_state() {
    let viewer = Viewer::default();
    assert!(viewer.title().is_empty());
    assert!(viewer.lines.is_empty());
    assert_eq!(viewer.scroll, ScrollTop::TOP);
}

#[test]
fn test_viewer_clone_is_independent() {
    let viewer = Viewer {
        scroll: ScrollTop::TOP.down(5, usize::MAX),
        ..Viewer::report("test", vec!["line1".to_owned(), "line2".to_owned()])
    };
    let mut cloned = viewer.clone();
    cloned.scroll = ScrollTop::TOP.down(10, usize::MAX);
    cloned.lines.push("line3".to_owned());

    assert_eq!(viewer.scroll.lines(), 5);
    assert_eq!(viewer.lines.len(), 2);
    assert_eq!(viewer.title(), "test");
}

// ---------------------------------------------------------------------------
// 9. format_size boundary values at exact thresholds
// ---------------------------------------------------------------------------

#[test]
fn test_format_size_exact_boundaries() {
    // Exactly at each boundary
    assert_eq!(FileSize::from_bytes(1024).to_string(), "1.00 KB");
    assert_eq!(FileSize::from_bytes(1024 * 1024).to_string(), "1.00 MB");
    assert_eq!(
        FileSize::from_bytes(1024 * 1024 * 1024).to_string(),
        "1.00 GB"
    );

    // One byte below each boundary
    assert_eq!(FileSize::from_bytes(1023).to_string(), "1023 B");
    assert_eq!(
        FileSize::from_bytes(1024 * 1024 - 1).to_string(),
        "1024.00 KB"
    );
    assert_eq!(
        FileSize::from_bytes(1024 * 1024 * 1024 - 1).to_string(),
        "1024.00 MB"
    );

    // Verify the suffix changes at boundaries
    assert!(FileSize::from_bytes(1023).to_string().ends_with(" B"));
    assert!(FileSize::from_bytes(1024).to_string().ends_with(" KB"));
    assert!(FileSize::from_bytes(1024 * 1024 - 1)
        .to_string()
        .ends_with(" KB"));
    assert!(FileSize::from_bytes(1024 * 1024)
        .to_string()
        .ends_with(" MB"));
    assert!(FileSize::from_bytes(1024 * 1024 * 1024 - 1)
        .to_string()
        .ends_with(" MB"));
    assert!(FileSize::from_bytes(1024 * 1024 * 1024)
        .to_string()
        .ends_with(" GB"));
}

#[test]
fn test_format_size_zero() {
    let result = FileSize::from_bytes(0).to_string();
    assert_eq!(result, "0 B");
    assert!(!result.is_empty());
}
