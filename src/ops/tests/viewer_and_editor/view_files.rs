//! Purpose: F3 on text, an empty file, invalid UTF-8, a folder and nothing.

use super::*;

#[test]
fn test_handle_view_file() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let file_path = temp_dir.path().join("test.txt");
    File::create(&file_path)
        .expect("test setup")
        .write_all(b"Line 1\nLine 2\nLine 3")
        .expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Select the file
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "test.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_view(&mut app).expect("test setup");

    assert!(app.get_viewer().is_some());
    let viewer = app.get_viewer().unwrap();
    assert_eq!(viewer.title(), "test.txt");
    assert_eq!(viewer.lines.len(), 3);
    assert_eq!(viewer.lines[0], "Line 1");
    assert!(app.status_text().contains("Viewing"));

    Ok(())
}

#[test]
fn test_handle_view_empty_file() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let file_path = temp_dir.path().join("empty.txt");
    File::create(&file_path).expect("test setup"); // Empty file

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Select the file
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "empty.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_view(&mut app).expect("test setup");

    assert!(app.get_viewer().is_some());
    let viewer = app.get_viewer().unwrap();
    assert_eq!(viewer.lines.len(), 1);
    assert_eq!(viewer.lines[0], "<empty file>");

    Ok(())
}

#[test]
fn test_handle_view_invalid_utf8() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let file_path = temp_dir.path().join("bad.txt");
    fs::write(&file_path, b"\xFF").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "bad.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    // Non-UTF-8 *text* is read in the editor's single-byte code page rather
    // than failing the whole action.
    handle_view(&mut app).expect("a non-UTF-8 decode must not fail");
    let viewer = app.get_viewer().expect("viewer must open");
    assert_eq!(viewer.lines, vec!["\u{44f}".to_owned()]);

    Ok(())
}

#[test]
fn test_handle_view_directory() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    fs::create_dir(temp_dir.path().join("subdir")).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Select the directory
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "subdir" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_view(&mut app).expect("test setup");

    assert!(app.get_viewer().is_none());
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

#[test]
fn test_handle_view_no_selection() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.left_panel.select(None);

    // Should not crash
    handle_view(&mut app).expect("test setup");
    assert!(app.get_viewer().is_none());

    Ok(())
}
