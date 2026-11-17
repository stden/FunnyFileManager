//! Purpose: unit tests for loading a listing and moving the cursor through it.

use super::*;

#[test]
fn test_panel_load_entries_with_files() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");

    // Create test files and directories
    fs::create_dir(temp_dir.path().join("dir1")).expect("test setup");
    fs::create_dir(temp_dir.path().join("dir2")).expect("test setup");
    File::create(temp_dir.path().join("file1.txt"))
        .expect("test setup")
        .write_all(b"test")
        .expect("test setup");
    File::create(temp_dir.path().join("file2.txt"))
        .expect("test setup")
        .write_all(b"test")
        .expect("test setup");

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    panel.load_entries().expect("test setup");

    assert_directories_first(&panel);
    assert!(panel.entries.len() >= 5);
    for (name, directory) in [
        ("dir1", true),
        ("dir2", true),
        ("file1.txt", false),
        ("file2.txt", false),
    ] {
        assert!(panel
            .entries
            .iter()
            .any(|entry| entry.name == name && entry.kind.is_dir() == directory));
    }

    Ok(())
}

fn assert_directories_first(panel: &Panel) {
    // Check entries are sorted (directories first, then files)
    let mut found_file = false;
    let mut found_dir_after_file = false;
    for entry in &panel.entries {
        if entry.name != ".." {
            found_dir_after_file |= entry.kind.is_dir() && found_file;
            found_file |= !entry.kind.is_dir();
        }
    }
    assert!(
        !found_dir_after_file,
        "Found directory after file in sorted entries"
    );
}

#[test]
fn test_panel_navigation_next() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");

    // Create multiple files
    for i in 0..5 {
        File::create(temp_dir.path().join(format!("file{i}.txt"))).expect("test setup");
    }

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Test next navigation
    assert_eq!(panel.selected(), Some(0));

    panel.next();
    assert_eq!(panel.selected(), Some(1));

    panel.next();
    assert_eq!(panel.selected(), Some(2));

    // Navigate to last item
    while panel.selected() != Some(panel.entries.len() - 1) {
        panel.next();
    }

    // Down at the bottom stays at the bottom. Wrapping would put the cursor
    // back on ".." and one more keypress would leave the directory — the
    // opposite of what holding Down was for.
    panel.next();
    assert_eq!(panel.selected(), Some(panel.entries.len() - 1));

    Ok(())
}

#[test]
fn test_panel_navigation_previous() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");

    // Create multiple files
    for i in 0..5 {
        File::create(temp_dir.path().join(format!("file{i}.txt"))).expect("test setup");
    }

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Up at the top stays at the top, the mirror of the rule below.
    panel.previous();
    assert_eq!(panel.selected(), Some(0));

    panel.select(Some(2));
    panel.previous();
    assert_eq!(panel.selected(), Some(1));

    Ok(())
}

#[test]
fn test_panel_navigation_empty() {
    let mut panel = Panel::from_entries(PathBuf::new(), Vec::new());

    // Navigation should not crash on empty panel
    panel.next();
    assert_eq!(panel.selected(), None);

    panel.previous();
    assert_eq!(panel.selected(), None);
}
