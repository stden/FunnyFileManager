//! Purpose: F3 on an empty file or a folder, F8 on `..`, and marked source paths.

use super::*;

// -----------------------------------------------------------------------
// handle_view edge cases
// -----------------------------------------------------------------------

#[test]
fn test_handle_view_empty_file_shows_placeholder() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    fs::write(temp_dir.path().join("empty.txt"), "")?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Select the empty file
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "empty.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_view(&mut app)?;
    let viewer = app.get_viewer().expect("viewer should be open");
    assert_eq!(viewer.lines.len(), 1);
    assert_eq!(viewer.lines[0], "<empty file>");

    Ok(())
}

#[test]
fn test_handle_view_directory_sets_status() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    fs::create_dir(temp_dir.path().join("mydir"))?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;

    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "mydir" && entry.kind.is_dir() {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_view(&mut app)?;
    assert!(
        app.get_viewer().is_none(),
        "viewer should not open for directory"
    );
    assert!(
        matches!(app.overlay, Overlay::None),
        "F3 on a directory measures it in-place without opening an overlay"
    );
    assert!(
        app.left_panel
            .get_selected_entry()
            .is_some_and(|entry| entry.directory_size.is_measured()),
        "F3 on a directory marks its size as measured"
    );

    Ok(())
}

// -----------------------------------------------------------------------
// handle_delete_confirm -- ".." selected should be a no-op
// -----------------------------------------------------------------------

#[test]
fn test_handle_delete_confirm_dotdot_is_noop() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let subdir = temp_dir.path().join("sub");
    fs::create_dir(&subdir)?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(subdir)?;

    // ".." should be the first entry
    assert_eq!(app.left_panel.entries[0].name, "..");
    app.left_panel.select(Some(0));

    handle_delete_confirm(&mut app)?;
    assert!(
        !matches!(app.overlay, Overlay::DeleteConfirm(_)),
        "delete confirm should not activate for '..'"
    );

    Ok(())
}

// -----------------------------------------------------------------------
// get_source_paths branches
// -----------------------------------------------------------------------

#[test]
fn test_get_source_paths_with_marks() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("a.txt"), "a")?;
    fs::write(temp_dir.path().join("b.txt"), "b")?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.left_panel.marked.insert(FileName::from("a.txt"));
    app.left_panel.marked.insert(FileName::from("b.txt"));

    let paths = get_source_paths(&app);
    assert_eq!(paths.len(), 2);

    Ok(())
}
