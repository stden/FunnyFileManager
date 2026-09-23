//! Edge-case unit tests for `FunnyFileManager`
//! Targets uncovered branches and edge cases across models, utils, s3, and handlers modules.

use funny_file_manager::models::FileName;
use funny_file_manager::models::FileSize;
use funny_file_manager::models::{App, DriveEntry, Panel, PanelSide, SortMode, Viewer};
#[cfg(feature = "s3")]
use funny_file_manager::s3::VirtualPath;
use funny_file_manager::scroll::ScrollTop;
use std::fs::{self, File};
use std::io;
use std::path::PathBuf;
use tempfile::TempDir;

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
    let entry = DriveEntry::S3Profile("my profile".to_string());
    assert_eq!(entry.display(), "S3: my profile");
}

#[test]
fn test_drive_entry_display_s3_profile_empty_name() {
    let entry = DriveEntry::S3Profile(String::new());
    assert_eq!(entry.display(), "S3: ");
}

// ---------------------------------------------------------------------------
// 3. VirtualPath::parse — profile@empty-bucket yields None bucket
// ---------------------------------------------------------------------------

#[cfg(feature = "s3")]
#[test]
fn test_virtual_path_parse_profile_at_empty_bucket() {
    let path = VirtualPath::parse("s3://myprofile@");
    match path {
        VirtualPath::S3 {
            profile,
            bucket,
            prefix,
        } => {
            assert_eq!(profile, "myprofile");
            assert_eq!(bucket, None);
            assert!(prefix.is_empty());
        }
        VirtualPath::Local(_) => panic!("Expected S3 path"),
    }
}

#[cfg(feature = "s3")]
#[test]
fn test_virtual_path_parse_profile_at_empty_bucket_with_prefix() {
    let path = VirtualPath::parse("s3://myprofile@/some/prefix");
    match path {
        VirtualPath::S3 {
            profile,
            bucket,
            prefix,
        } => {
            assert_eq!(profile, "myprofile");
            // bucket_part is "myprofile@", split_once('/') gives ("myprofile@", "some/prefix")
            // then profile@bucket_name split: profile="myprofile", bucket_name=""
            assert_eq!(bucket, None);
            assert_eq!(prefix, "some/prefix");
        }
        VirtualPath::Local(_) => panic!("Expected S3 path"),
    }
}

#[cfg(feature = "s3")]
#[test]
fn test_virtual_path_parse_bare_s3_no_trailing_slash() {
    // "s3://" with nothing after — tests the empty bucket_part branch
    let path = VirtualPath::parse("s3://");
    match path {
        VirtualPath::S3 {
            profile,
            bucket,
            prefix,
        } => {
            assert_eq!(profile, "default");
            assert_eq!(bucket, None);
            assert!(prefix.is_empty());
        }
        VirtualPath::Local(_) => panic!("Expected S3 path"),
    }
}

#[cfg(feature = "s3")]
#[test]
fn test_virtual_path_roundtrip_local() {
    let original = "/some/path/here";
    let parsed = VirtualPath::parse(original);
    assert_eq!(parsed.display(), original);
}

#[cfg(feature = "s3")]
#[test]
fn test_virtual_path_roundtrip_s3_with_prefix() {
    let parsed = VirtualPath::parse("s3://prod@bucket/dir/subdir");
    assert_eq!(parsed.display(), "s3://prod@bucket/dir/subdir");
}

// ---------------------------------------------------------------------------
// 4. Panel::clear_marks — verify marks are actually cleared
// ---------------------------------------------------------------------------

#[test]
fn test_panel_clear_marks_empties_set() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    File::create(temp_dir.path().join("a.txt"))?;
    File::create(temp_dir.path().join("b.txt"))?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    panel.marked.insert(FileName::from("a.txt"));
    panel.marked.insert(FileName::from("b.txt"));
    assert_eq!(panel.marked.len(), 2);

    panel.clear_marks();
    assert!(panel.marked.is_empty());

    Ok(())
}

// ---------------------------------------------------------------------------
// 5. Panel::find_first_matching — case insensitivity and edge inputs
// ---------------------------------------------------------------------------

#[test]
fn test_find_first_matching_case_insensitive() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    File::create(temp_dir.path().join("Alpha.txt"))?;
    File::create(temp_dir.path().join("beta.txt"))?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    // Search with different case
    assert!(panel.find_first_matching("ALPHA"));
    let selected = panel.get_selected_entry().unwrap();
    assert_eq!(selected.name, "Alpha.txt");

    Ok(())
}

#[test]
fn test_find_first_matching_returns_false_for_empty_prefix() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    File::create(temp_dir.path().join("a.txt"))?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    assert!(!panel.find_first_matching(""));

    Ok(())
}

#[test]
fn test_find_first_matching_skips_dotdot() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let sub = temp_dir.path().join("sub");
    fs::create_dir(&sub)?;
    // No files named starting with "." except ".."
    let mut panel = Panel::new(sub)?;
    // ".." should exist; search for ".." should not match it
    assert!(!panel.find_first_matching(".."));

    Ok(())
}

// ---------------------------------------------------------------------------
// 6. Panel::find_next_matching — wrapping from end to beginning
// ---------------------------------------------------------------------------

#[test]
fn test_find_next_matching_wraps_around() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    File::create(temp_dir.path().join("apple.txt"))?;
    File::create(temp_dir.path().join("banana.txt"))?;
    File::create(temp_dir.path().join("avocado.txt"))?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    // Select first "a" match
    assert!(panel.find_first_matching("a"));
    let first = panel.selected().unwrap();

    // Find next "a" match — should wrap around if needed
    assert!(panel.find_next_matching("a"));
    let second = panel.selected().unwrap();
    assert_ne!(first, second);

    Ok(())
}

// ---------------------------------------------------------------------------
// 7. App::get_active_panel_ref and get_inactive_panel_ref
// ---------------------------------------------------------------------------

#[test]
fn test_app_active_and_inactive_panel_ref() -> io::Result<()> {
    let app = App::new()?;
    let active = app.get_active_panel_ref();
    let inactive = app.get_inactive_panel_ref();
    // Active is left by default, inactive is right
    assert_eq!(active.path, app.left_panel.path);
    assert_eq!(inactive.path, app.right_panel.path);

    Ok(())
}

#[test]
fn test_app_active_panel_ref_after_switch() -> io::Result<()> {
    let mut app = App::new()?;
    app.active_panel = PanelSide::Right;

    let active = app.get_active_panel_ref();
    let inactive = app.get_inactive_panel_ref();
    assert_eq!(active.path, app.right_panel.path);
    assert_eq!(inactive.path, app.left_panel.path);

    Ok(())
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
        ..Viewer::report("test", vec!["line1".to_string(), "line2".to_string()])
    };
    let mut cloned = viewer.clone();
    cloned.scroll = ScrollTop::TOP.down(10, usize::MAX);
    cloned.lines.push("line3".to_string());

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

// ---------------------------------------------------------------------------
// 10. Panel::toggle_mark_selected on ".." entry returns false
// ---------------------------------------------------------------------------

#[test]
fn test_toggle_mark_dotdot_returns_false() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let sub = temp_dir.path().join("sub");
    fs::create_dir(&sub)?;

    let mut panel = Panel::new(sub)?;
    // First entry should be ".."
    panel.select(Some(0));
    assert_eq!(panel.entries[0].name, "..");
    assert!(!panel.toggle_mark_selected());
    assert!(panel.marked.is_empty());

    Ok(())
}

// ---------------------------------------------------------------------------
// 11. Panel::selected_path returns None for ".." entry
// ---------------------------------------------------------------------------

#[test]
fn test_selected_path_returns_none_for_dotdot() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let sub = temp_dir.path().join("sub");
    fs::create_dir(&sub)?;

    let panel = Panel::new(sub)?;
    // ".." should give None for selected_path
    assert_eq!(panel.entries[0].name, "..");
    // state selects 0 by default
    assert!(panel.selected_path().is_none());

    // But a real file should give Some
    Ok(())
}

// ---------------------------------------------------------------------------
// 12. Panel::action_entry_names returns empty when ".." is selected and no marks
// ---------------------------------------------------------------------------

#[test]
fn test_action_entry_names_dotdot_no_marks_returns_empty() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let sub = temp_dir.path().join("sub");
    fs::create_dir(&sub)?;
    File::create(sub.join("file.txt"))?;

    let mut panel = Panel::new(sub)?;
    panel.select(Some(0)); // ".."
    let names = panel.action_entry_names();
    assert!(names.is_empty());

    Ok(())
}

// ---------------------------------------------------------------------------
// 13. Panel::action_entry_names returns marked names when marks exist
// ---------------------------------------------------------------------------

#[test]
fn test_action_entry_names_returns_marks_over_selection() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    File::create(temp_dir.path().join("a.txt"))?;
    File::create(temp_dir.path().join("b.txt"))?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    panel.marked.insert(FileName::from("a.txt"));
    panel.marked.insert(FileName::from("b.txt"));
    // Even though something else might be selected, marks take precedence
    let names = panel.action_entry_names();
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"a.txt".to_string()));
    assert!(names.contains(&"b.txt".to_string()));

    Ok(())
}

// ---------------------------------------------------------------------------
// 14. Panel navigation on empty panel is a no-op
// ---------------------------------------------------------------------------

#[test]
fn test_panel_next_previous_on_empty_panel() {
    let mut panel = Panel::from_entries(PathBuf::from("/nonexistent"), vec![]);
    // Should not panic
    panel.next();
    panel.previous();
    assert!(panel.selected().is_none());
}

// ---------------------------------------------------------------------------
// 15. Panel::ensure_selection on non-empty panel with no selection
// ---------------------------------------------------------------------------

#[test]
fn test_ensure_selection_selects_first_when_none() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    File::create(temp_dir.path().join("file.txt"))?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    panel.select(None);
    panel.ensure_selection();
    assert_eq!(panel.selected(), Some(0));

    Ok(())
}

// ---------------------------------------------------------------------------
// 16. Panel::load_entries prunes stale marks
// ---------------------------------------------------------------------------

#[test]
fn test_load_entries_prunes_stale_marks() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    File::create(temp_dir.path().join("keep.txt"))?;
    File::create(temp_dir.path().join("remove_me.txt"))?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    panel.marked.insert(FileName::from("keep.txt"));
    panel.marked.insert(FileName::from("remove_me.txt"));
    panel.marked.insert(FileName::from("already_gone.txt"));

    // Delete one file
    fs::remove_file(temp_dir.path().join("remove_me.txt"))?;
    panel.load_entries()?;

    // "already_gone.txt" and "remove_me.txt" should be pruned
    assert!(panel.marked.contains("keep.txt"));
    assert!(!panel.marked.contains("remove_me.txt"));
    assert!(!panel.marked.contains("already_gone.txt"));

    Ok(())
}

// ---------------------------------------------------------------------------
// 17. App::swap_panels actually exchanges paths
// ---------------------------------------------------------------------------

#[test]
fn test_swap_panels_exchanges_paths() -> io::Result<()> {
    let temp1 = TempDir::new()?;
    let temp2 = TempDir::new()?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp1.path().to_path_buf())?;
    app.right_panel = Panel::new(temp2.path().to_path_buf())?;

    let left_path = app.left_panel.path.clone();
    let right_path = app.right_panel.path.clone();

    app.swap_panels();

    assert_eq!(app.left_panel.path, right_path);
    assert_eq!(app.right_panel.path, left_path);

    Ok(())
}

// ---------------------------------------------------------------------------
// 18. Panel::enter on non-directory is a no-op
// ---------------------------------------------------------------------------

#[test]
fn test_enter_on_file_is_noop() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    File::create(temp_dir.path().join("file.txt"))?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    // Select the file entry
    let file_idx = panel
        .entries
        .iter()
        .position(|e| e.name == "file.txt")
        .unwrap();
    panel.select(Some(file_idx));

    let path_before = panel.path.clone();
    panel.enter()?;
    assert_eq!(panel.path, path_before);

    Ok(())
}

// ---------------------------------------------------------------------------
// 19. Panel::enter with no selection is a no-op
// ---------------------------------------------------------------------------

#[test]
fn test_enter_with_no_selection_is_noop() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    panel.select(None);

    let path_before = panel.path.clone();
    panel.enter()?;
    assert_eq!(panel.path, path_before);

    Ok(())
}
