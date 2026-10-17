//! Purpose: F3 error paths: nothing selected, a folder and a missing file.

use super::*;

#[test]
fn test_handle_view_error_paths() -> io::Result<()> {
    let temp_dir_outer = TempDir::new().expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir_outer.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir_outer.path().to_path_buf())?;

    // No selection
    handle_view(&mut app).expect("test setup");
    assert!(app.get_viewer().is_none());

    // Use the existing temp_dir_outer
    app.left_panel.path = temp_dir_outer.path().to_path_buf();

    // Non-existent directory (selected but missing on disk)
    select_missing_file(&mut app.left_panel);
    // This should fail because missing.txt doesn't exist on disk
    assert!(handle_view(&mut app).is_err());

    // View a directory (that exists)
    let subdir = temp_dir_outer.path().join("subdir");
    fs::create_dir(&subdir).expect("test setup");
    app.left_panel.load_entries().expect("test setup");

    // Find subdir in entries and select it
    assert!(app.left_panel.select_named("subdir"));

    handle_view(&mut app).expect("test setup");
    assert!(
        matches!(app.overlay, Overlay::Information(_)),
        "F3 on a directory measures it, as Far does"
    );

    Ok(())
}

fn select_missing_file(panel: &mut Panel) {
    panel.entries.clear();
    panel.entries.push(crate::models::DirEntry {
        name: "..".to_owned(),
        kind: EntryKind::Directory,
        size: FileSize::from_bytes(0),
        modified: Modified::UNKNOWN,
        executable: false,
        ..crate::models::DirEntry::default()
    });
    panel.entries.push(crate::models::DirEntry {
        name: "missing.txt".to_owned(),
        kind: EntryKind::File,
        size: FileSize::from_bytes(0),
        modified: Modified::UNKNOWN,
        executable: false,
        ..crate::models::DirEntry::default()
    });
    panel.select(Some(1));
}
