//! Purpose: F6 across devices: copy, then remove the source.

use super::*;

#[test]
fn test_handle_move_cross_device() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");
    fs::create_dir(&left_dir).expect("test setup");
    fs::create_dir(&right_dir).expect("test setup");

    // Create a directory to move
    let source_dir = left_dir.join("move_dir");
    fs::create_dir(&source_dir).expect("test setup");
    File::create(source_dir.join("file.txt"))
        .expect("test setup")
        .write_all(b"content")
        .expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(left_dir).expect("test setup");
    app.right_panel = Panel::new(right_dir.clone()).expect("test setup");

    // Select the directory
    assert!(app.left_panel.select_named("move_dir"));

    let _guard = ForceRenameFailureGuard::enable();
    handle_move(&mut app).expect("test setup");

    assert!(!source_dir.exists());
    assert!(right_dir.join("move_dir").exists());
    assert!(right_dir.join("move_dir").join("file.txt").exists());
    assert!(app.status_text().contains("Moved to"));

    Ok(())
}

#[test]
fn test_handle_move_fallback_for_file() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");
    fs::create_dir(&left_dir).expect("test setup");
    fs::create_dir(&right_dir).expect("test setup");

    let file_name = "move_file.txt";
    let source_file = left_dir.join(file_name);
    File::create(&source_file)
        .expect("test setup")
        .write_all(b"content")
        .expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(left_dir).expect("test setup");
    app.right_panel = Panel::new(right_dir.clone()).expect("test setup");

    assert!(app.left_panel.select_named(file_name));

    let _guard = ForceRenameFailureGuard::enable();
    handle_move(&mut app).expect("test setup");

    assert!(!source_file.exists());
    let moved_file = right_dir.join(file_name);
    assert!(moved_file.exists());
    assert_eq!(
        fs::read_to_string(&moved_file).expect("test setup"),
        "content"
    );
    assert!(app.status_text().contains("Moved to"));

    Ok(())
}
