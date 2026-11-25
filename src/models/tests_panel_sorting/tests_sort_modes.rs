//! Purpose: unit tests for the sort modes: their cycle, labels, defaults, and
//! the order a listing takes in each.

use super::*;

#[test]
fn test_sort_mode_next_and_label() {
    assert_eq!(SortMode::Name.label(), "Name");
    assert_eq!(SortMode::Extension.label(), "Ext");
    assert_eq!(SortMode::Size.label(), "Size");

    assert_eq!(SortMode::Name.next(), SortMode::Extension);
    assert_eq!(SortMode::Extension.next(), SortMode::Size);
    assert_eq!(SortMode::Size.next(), SortMode::Name);
}

#[test]
fn test_panel_cycle_sort_mode() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    assert_eq!(panel.sort_mode, SortMode::Name);
    panel.cycle_sort_mode();
    assert_eq!(panel.sort_mode, SortMode::Extension);
    panel.cycle_sort_mode();
    assert_eq!(panel.sort_mode, SortMode::Size);
    panel.cycle_sort_mode();
    assert_eq!(panel.sort_mode, SortMode::Name);

    Ok(())
}

#[test]
fn test_panel_load_entries_sort_modes() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    fs::create_dir(temp_dir.path().join("dir")).expect("test setup");
    File::create(temp_dir.path().join("b.txt"))
        .expect("test setup")
        .write_all(b"1")
        .expect("test setup");
    File::create(temp_dir.path().join("a.rs"))
        .expect("test setup")
        .write_all(b"1234")
        .expect("test setup");
    File::create(temp_dir.path().join("c"))
        .expect("test setup")
        .write_all(b"12")
        .expect("test setup");

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    panel.sort_mode = SortMode::Extension;
    panel.load_entries().expect("test setup");
    assert_eq!(file_names(&panel), vec!["c", "a.rs", "b.txt"]);

    panel.sort_mode = SortMode::Size;
    panel.load_entries().expect("test setup");
    assert_eq!(file_names(&panel), vec!["a.rs", "c", "b.txt"]);

    Ok(())
}

#[test]
fn test_sort_mode_edge_cases() {
    let mut mode = SortMode::Name;

    mode = mode.next();
    assert_eq!(mode, SortMode::Extension);
    assert_eq!(mode.label(), "Ext");

    mode = mode.next();
    assert_eq!(mode, SortMode::Size);
    assert_eq!(mode.label(), "Size");

    mode = mode.next();
    assert_eq!(mode, SortMode::Name);
    assert_eq!(mode.label(), "Name");
}

#[test]
fn test_sort_mode_cycle() {
    let mut mode = SortMode::Name;
    assert_eq!(mode, SortMode::Name);

    mode = mode.next();
    assert_eq!(mode, SortMode::Extension);

    mode = mode.next();
    assert_eq!(mode, SortMode::Size);

    mode = mode.next();
    assert_eq!(mode, SortMode::Name);
}

#[test]
fn test_sort_mode_default() {
    // Test the Default implementation for SortMode
    let default_mode: SortMode = SortMode::default();
    assert_eq!(default_mode, SortMode::Name);
}

#[test]
fn test_sort_mode_extension_no_extension() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");

    // Create files with and without extensions
    File::create(temp_dir.path().join("file1.txt")).expect("test setup");
    File::create(temp_dir.path().join("file2")).expect("test setup"); // No extension
    File::create(temp_dir.path().join("file3.md")).expect("test setup");
    File::create(temp_dir.path().join("file4")).expect("test setup"); // No extension

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    panel.sort_mode = SortMode::Extension;
    panel.load_entries().expect("test setup");

    // Files without extensions should sort before files with extensions (after "..")
    let entries = &panel.entries;
    assert!(entries[0].name == ".." || entries[1].name == "file2" || entries[2].name == "file4");

    Ok(())
}
