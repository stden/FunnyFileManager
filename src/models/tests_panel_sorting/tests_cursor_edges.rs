//! Purpose: unit tests for the cursor at the edges: no selection, both ends,
//! a large or empty listing, and the parent directory.

use super::*;

#[test]
fn test_panel_next_with_none_selected() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    File::create(temp_dir.path().join("test.txt")).expect("test setup");

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Clear the selection
    panel.select(None);

    // Next should select the first item
    panel.next();
    assert_eq!(panel.selected(), Some(0));

    Ok(())
}

#[test]
fn test_panel_previous_with_none_selected() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    File::create(temp_dir.path().join("test.txt")).expect("test setup");

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Clear the selection
    panel.select(None);

    // Previous should select the first item
    panel.previous();
    assert_eq!(panel.selected(), Some(0));

    Ok(())
}

/// Navigation stops at the ends rather than wrapping.
#[test]
fn test_panel_navigation_stops_at_both_ends() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    for i in 0..3 {
        File::create(temp_dir.path().join(format!("file{i}.txt"))).expect("test setup");
    }

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    panel.load_entries().expect("test setup");

    // With 3 files plus ".." entry, we have indices 0-3
    panel.select(Some(0));
    panel.previous();
    assert_eq!(panel.selected(), Some(0), "up at the top stays");

    panel.select(Some(3));
    panel.next();
    assert_eq!(panel.selected(), Some(3), "down at the bottom stays");

    Ok(())
}

#[test]
fn test_panel_with_max_entries() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");

    for i in 0..1000 {
        File::create(temp_dir.path().join(format!("file{i:04}.txt"))).expect("test setup");
    }

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    panel.load_entries().expect("test setup");

    assert_eq!(panel.entries.len(), 1001);

    panel.select(Some(0));
    for _ in 0..500 {
        panel.next();
    }
    assert_eq!(panel.selected(), Some(500));

    panel.select(Some(1000));
    let entry_name = panel.entries[1000].name.clone();
    assert_eq!(panel.selected_path(), Some(panel.path.join(entry_name)));

    Ok(())
}

#[test]
fn test_panel_empty_directory_operations() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let empty_dir = temp_dir.path().join("empty");
    fs::create_dir(&empty_dir).expect("test setup");

    let mut panel = Panel::new(empty_dir).expect("test setup");
    panel.load_entries().expect("test setup");

    assert_eq!(panel.entries.len(), 1);
    assert_eq!(panel.entries[0].name, "..");

    panel.next();
    assert_eq!(panel.selected(), Some(0));

    panel.previous();
    assert_eq!(panel.selected(), Some(0));

    panel.toggle_mark_selected();
    assert!(panel.marked.is_empty());

    Ok(())
}

#[test]
fn test_panel_enter_parent_directory_navigation() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let subdir = temp_dir.path().join("subdir");
    fs::create_dir(&subdir).expect("test setup");

    let mut panel = Panel::new(subdir).expect("test setup");
    panel.load_entries().expect("test setup");

    // Select parent directory ".."
    panel.select(Some(0));
    let selected_entry = panel.get_selected_entry().unwrap();
    assert_eq!(selected_entry.name, "..");

    // Enter parent directory
    panel.enter().expect("test setup");

    // Should now be in the parent directory
    assert_eq!(
        panel.path.canonicalize().expect("test setup"),
        temp_dir.path().canonicalize().expect("test setup")
    );

    Ok(())
}
