//! Purpose: Generates varied utility scenarios for copy, refresh, and unique path logic.
//!
use funny_file_manager::models::App;
use funny_file_manager::models::FileSize;
use funny_file_manager::utils::{copy_dir_all, refresh_panels, unique_destination};
use std::ffi::OsStr;
use std::fs;
use std::io;
use tempfile::TempDir;

#[test]
fn format_size_stays_in_bytes_below_kib() {
    assert_eq!(FileSize::from_bytes(1023).to_string(), "1023 B");
}

#[test]
fn format_size_switches_to_kib_at_boundary() {
    assert_eq!(FileSize::from_bytes(1024).to_string(), "1.00 KB");
}

#[test]
fn format_size_formats_fractional_kib() {
    assert_eq!(FileSize::from_bytes(1536).to_string(), "1.50 KB");
}

#[test]
fn format_size_switches_to_mib_at_boundary() {
    assert_eq!(FileSize::from_bytes(1024 * 1024).to_string(), "1.00 MB");
}

#[test]
fn format_size_formats_fractional_mib() {
    assert_eq!(
        FileSize::from_bytes(5 * 1024 * 1024 + 512 * 1024).to_string(),
        "5.50 MB"
    );
}

#[test]
fn format_size_switches_to_gib_at_boundary() {
    assert_eq!(
        FileSize::from_bytes(1024 * 1024 * 1024).to_string(),
        "1.00 GB"
    );
}

#[test]
fn unique_destination_returns_original_when_free() -> io::Result<()> {
    let temp = TempDir::new()?;
    let path = unique_destination(temp.path(), OsStr::new("report.txt"))?;
    assert_eq!(path, temp.path().join("report.txt"));
    Ok(())
}

#[test]
fn unique_destination_adds_suffix_for_existing_file() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("report.txt"), b"x")?;

    let path = unique_destination(temp.path(), OsStr::new("report.txt"))?;

    assert_eq!(path, temp.path().join("report_1.txt"));
    Ok(())
}

#[test]
fn unique_destination_skips_to_next_available_suffix() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("report.txt"), b"x")?;
    fs::write(temp.path().join("report_1.txt"), b"x")?;
    fs::write(temp.path().join("report_2.txt"), b"x")?;

    let path = unique_destination(temp.path(), OsStr::new("report.txt"))?;

    assert_eq!(path, temp.path().join("report_3.txt"));
    Ok(())
}

#[test]
fn unique_destination_handles_names_without_extension() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("archive"), b"x")?;

    let path = unique_destination(temp.path(), OsStr::new("archive"))?;

    assert_eq!(path, temp.path().join("archive_1"));
    Ok(())
}

#[test]
fn unique_destination_handles_hidden_files() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join(".env"), b"x")?;

    let path = unique_destination(temp.path(), OsStr::new(".env"))?;

    assert_eq!(path, temp.path().join(".env_1"));
    Ok(())
}

#[test]
fn copy_dir_all_copies_nested_files() -> io::Result<()> {
    let temp = TempDir::new()?;
    let src = temp.path().join("src");
    let dst = temp.path().join("dst");
    fs::create_dir_all(src.join("nested/deeper"))?;
    fs::write(src.join("root.txt"), b"root")?;
    fs::write(src.join("nested/file.txt"), b"nested")?;
    fs::write(src.join("nested/deeper/end.txt"), b"end")?;

    copy_dir_all(&src, &dst)?;

    assert_eq!(fs::read(dst.join("root.txt"))?, b"root");
    assert_eq!(fs::read(dst.join("nested/file.txt"))?, b"nested");
    assert_eq!(fs::read(dst.join("nested/deeper/end.txt"))?, b"end");
    Ok(())
}

#[test]
fn copy_dir_all_creates_destination_tree() -> io::Result<()> {
    let temp = TempDir::new()?;
    let src = temp.path().join("source");
    let dst = temp.path().join("missing/target");
    fs::create_dir_all(src.join("subdir"))?;
    fs::write(src.join("subdir/file.txt"), b"data")?;

    copy_dir_all(&src, &dst)?;

    assert!(dst.join("subdir").is_dir());
    assert!(dst.join("subdir/file.txt").is_file());
    Ok(())
}

#[test]
fn refresh_panels_keeps_selection_valid_after_file_changes() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("alpha.txt"), b"a")?;
    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.right_panel.path = temp.path().to_path_buf();

    refresh_panels(&mut app)?;
    assert_eq!(app.left_panel.selected(), Some(0));

    fs::write(temp.path().join("beta.txt"), b"b")?;
    refresh_panels(&mut app)?;

    assert!(app
        .left_panel
        .entries
        .iter()
        .any(|entry| entry.name == "beta.txt"));
    assert!(app
        .right_panel
        .entries
        .iter()
        .any(|entry| entry.name == "beta.txt"));
    assert!(app.left_panel.selected().is_some());
    assert!(app.right_panel.selected().is_some());
    Ok(())
}

#[test]
fn refresh_panels_handles_empty_directory() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.right_panel.path = temp.path().to_path_buf();

    refresh_panels(&mut app)?;

    assert!(app
        .left_panel
        .entries
        .iter()
        .any(|entry| entry.name == ".."));
    assert!(app
        .right_panel
        .entries
        .iter()
        .any(|entry| entry.name == ".."));
    Ok(())
}
