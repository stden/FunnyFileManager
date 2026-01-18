use funny_file_manager::models::App;
use funny_file_manager::utils::{copy_dir_all, format_size, refresh_panels, unique_destination};
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_format_size_not_empty() {
    // Kill: replace format_size -> String with String::new()
    let result = format_size(12345);
    assert!(!result.is_empty());
    assert!(result.len() > 0);
}

#[test]
fn test_format_size_not_xyzzy() {
    // Kill: replace format_size -> String with "xyzzy".into()
    let result = format_size(5000);
    assert!(!result.contains("xyzzy"));
    assert!(result.contains("KB") || result.contains("B"));
}

#[test]
fn test_format_size_kb_multiplication() {
    // Kill: replace * with + in KB = 1024
    // Kill: replace * with / in KB = 1024
    // If KB was 1024 + 1024 = 2048, then 1024 bytes would show as 0.5 KB
    // If KB was 1024 / 1024 = 1, then 1024 bytes would show as 1024.00 KB
    assert_eq!(format_size(1024), "1.00 KB");
    assert_eq!(format_size(2048), "2.00 KB");
    assert_ne!(format_size(1024), "0.50 KB");
    assert_ne!(format_size(1024), "1024.00 KB");
}

#[test]
fn test_format_size_mb_multiplication() {
    // Kill: replace * with + in MB = KB * 1024
    // Kill: replace * with / in MB = KB * 1024
    // MB should be 1048576, not 1024 + 1024 = 2048
    assert_eq!(format_size(1048576), "1.00 MB");
    assert_eq!(format_size(2097152), "2.00 MB");
    // If MB was KB + 1024 = 2048, then 1MB would show differently
    assert_ne!(format_size(2048), "1.00 MB");
    assert_ne!(format_size(1048576), "512.00 MB");
}

#[test]
fn test_format_size_gb_comparison() {
    // Kill: replace >= with < in if size >= GB
    let gb_minus_1 = 1073741823;
    let gb_exact = 1073741824;
    let gb_plus_1 = 1073741825;

    assert!(format_size(gb_minus_1).contains("MB"));
    assert!(format_size(gb_exact).contains("GB"));
    assert!(format_size(gb_plus_1).contains("GB"));

    // These should be different
    assert_ne!(format_size(gb_minus_1), format_size(gb_exact));
}

#[test]
fn test_format_size_mb_comparison() {
    // Kill: replace >= with < in if size >= MB
    let mb_minus_1 = 1048575;
    let mb_exact = 1048576;
    let mb_plus_1 = 1048577;

    assert!(format_size(mb_minus_1).contains("KB"));
    assert!(format_size(mb_exact).contains("MB"));
    assert!(format_size(mb_plus_1).contains("MB"));

    // These should be different
    assert_ne!(format_size(mb_minus_1), format_size(mb_exact));
}

#[test]
fn test_format_size_kb_comparison() {
    // Kill: replace >= with < in if size >= KB
    let kb_minus_1 = 1023;
    let kb_exact = 1024;
    let kb_plus_1 = 1025;

    assert!(format_size(kb_minus_1).contains("B") && !format_size(kb_minus_1).contains("KB"));
    assert!(format_size(kb_exact).contains("KB"));
    assert!(format_size(kb_plus_1).contains("KB"));

    // These should be different
    assert_ne!(format_size(kb_minus_1), format_size(kb_exact));
}

#[test]
fn test_unique_destination_not_default() -> io::Result<()> {
    // Kill: replace unique_destination -> PathBuf with Default::default()
    let temp_dir = TempDir::new()?;
    let result = unique_destination(temp_dir.path(), OsStr::new("test.txt"));

    assert!(!result.as_os_str().is_empty());
    assert_ne!(result, PathBuf::default());
    assert_ne!(result, PathBuf::new());
    assert!(result.starts_with(temp_dir.path()));

    Ok(())
}

#[test]
fn test_unique_destination_extension_guard() -> io::Result<()> {
    // Kill: replace match guard !ext.is_empty() with true
    // Kill: replace match guard !ext.is_empty() with false
    // Kill: delete ! in unique_destination
    let temp_dir = TempDir::new()?;

    // Test with empty extension
    File::create(temp_dir.path().join("empty."))?;
    let unique_empty_ext = unique_destination(temp_dir.path(), OsStr::new("empty."));
    assert!(unique_empty_ext.to_string_lossy().ends_with("empty_1"));
    assert!(!unique_empty_ext.to_string_lossy().ends_with("empty_1."));

    // Test with non-empty extension
    File::create(temp_dir.path().join("file.txt"))?;
    let unique_with_ext = unique_destination(temp_dir.path(), OsStr::new("file.txt"));
    assert!(unique_with_ext.to_string_lossy().ends_with("file_1.txt"));

    // Test the difference between empty and non-empty extensions
    assert!(unique_with_ext.to_string_lossy().contains("."));
    assert!(!unique_empty_ext.to_string_lossy().ends_with("."));

    Ok(())
}

#[test]
fn test_unique_destination_counter_increment() -> io::Result<()> {
    // Kill: replace += with -= in counter increment
    // Kill: replace += with *= in counter increment
    let temp_dir = TempDir::new()?;

    // Create a series of files
    File::create(temp_dir.path().join("seq.txt"))?;
    File::create(temp_dir.path().join("seq_1.txt"))?;
    File::create(temp_dir.path().join("seq_2.txt"))?;
    File::create(temp_dir.path().join("seq_3.txt"))?;

    let unique = unique_destination(temp_dir.path(), OsStr::new("seq.txt"));

    // Should be seq_4.txt (counter incremented by 1)
    assert!(unique.to_string_lossy().contains("seq_4.txt"));

    // Should NOT be negative (would happen with -=)
    assert!(!unique.to_string_lossy().contains("-"));

    // Should NOT be exponential (would happen with *=)
    assert!(!unique.to_string_lossy().contains("seq_6.txt")); // 3 * 2
    assert!(!unique.to_string_lossy().contains("seq_9.txt")); // 3 * 3

    Ok(())
}

#[test]
fn test_copy_dir_all_actually_copies() -> io::Result<()> {
    // Kill: replace copy_dir_all -> io::Result<()> with Ok(())
    let temp_dir = TempDir::new()?;
    let src = temp_dir.path().join("src");
    let dst = temp_dir.path().join("dst");

    // Create source structure
    fs::create_dir(&src)?;
    File::create(src.join("file1.txt"))?.write_all(b"content1")?;
    fs::create_dir(src.join("subdir"))?;
    File::create(src.join("subdir").join("file2.txt"))?.write_all(b"content2")?;

    // Perform copy
    copy_dir_all(&src, &dst)?;

    // Verify files were actually copied
    assert!(dst.exists(), "Destination directory should exist");
    assert!(dst.is_dir(), "Destination should be a directory");

    let file1_content = fs::read_to_string(dst.join("file1.txt"))?;
    assert_eq!(file1_content, "content1", "File content should match");

    assert!(dst.join("subdir").exists(), "Subdirectory should be copied");
    let file2_content = fs::read_to_string(dst.join("subdir").join("file2.txt"))?;
    assert_eq!(
        file2_content, "content2",
        "Nested file content should match"
    );

    Ok(())
}

#[test]
fn test_copy_dir_all_empty_dir() -> io::Result<()> {
    // Additional test to ensure empty directories are handled
    let temp_dir = TempDir::new()?;
    let src = temp_dir.path().join("empty_src");
    let dst = temp_dir.path().join("empty_dst");

    fs::create_dir(&src)?;
    copy_dir_all(&src, &dst)?;

    assert!(dst.exists());
    assert!(dst.is_dir());
    assert_eq!(fs::read_dir(&dst)?.count(), 0);

    Ok(())
}

#[test]
fn test_refresh_panels_actually_refreshes() -> io::Result<()> {
    // Kill: replace refresh_panels -> io::Result<()> with Ok(())
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;

    // Set both panels to the temp directory
    app.left_panel.path = temp_dir.path().to_path_buf();
    app.right_panel.path = temp_dir.path().to_path_buf();

    // Initial load
    app.left_panel.load_entries()?;
    app.right_panel.load_entries()?;
    let initial_left_count = app.left_panel.entries.len();
    let initial_right_count = app.right_panel.entries.len();

    // Add multiple new files
    File::create(temp_dir.path().join("new_file_1.txt"))?;
    File::create(temp_dir.path().join("new_file_2.txt"))?;
    fs::create_dir(temp_dir.path().join("new_dir"))?;

    // Refresh panels
    refresh_panels(&mut app)?;

    // Verify panels were refreshed with new content
    assert!(
        app.left_panel.entries.len() > initial_left_count,
        "Left panel should have more entries after refresh"
    );
    assert!(
        app.right_panel.entries.len() > initial_right_count,
        "Right panel should have more entries after refresh"
    );

    // Verify specific new entries exist
    let left_names: Vec<_> = app.left_panel.entries.iter().map(|e| &e.name).collect();
    assert!(left_names.contains(&&"new_file_1.txt".to_string()));
    assert!(left_names.contains(&&"new_file_2.txt".to_string()));
    assert!(left_names.contains(&&"new_dir".to_string()));

    let right_names: Vec<_> = app.right_panel.entries.iter().map(|e| &e.name).collect();
    assert!(right_names.contains(&&"new_file_1.txt".to_string()));
    assert!(right_names.contains(&&"new_file_2.txt".to_string()));
    assert!(right_names.contains(&&"new_dir".to_string()));

    // Verify selections are maintained (ensure_selection is called)
    assert!(app.left_panel.state.selected().is_some());
    assert!(app.right_panel.state.selected().is_some());

    Ok(())
}

#[test]
fn test_refresh_panels_handles_deleted_files() -> io::Result<()> {
    // Additional test for refresh behavior with deleted files
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;

    app.left_panel.path = temp_dir.path().to_path_buf();
    app.right_panel.path = temp_dir.path().to_path_buf();

    // Create initial files
    File::create(temp_dir.path().join("temp1.txt"))?;
    File::create(temp_dir.path().join("temp2.txt"))?;

    // Initial load
    refresh_panels(&mut app)?;
    let initial_count = app.left_panel.entries.len();

    // Delete a file
    fs::remove_file(temp_dir.path().join("temp1.txt"))?;

    // Refresh again
    refresh_panels(&mut app)?;

    // Verify the file list was updated
    assert!(app.left_panel.entries.len() < initial_count);
    let names: Vec<_> = app.left_panel.entries.iter().map(|e| &e.name).collect();
    assert!(!names.contains(&&"temp1.txt".to_string()));
    assert!(names.contains(&&"temp2.txt".to_string()));

    Ok(())
}
