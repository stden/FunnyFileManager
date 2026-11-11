//! Purpose: F6 as a rename, and the free name it picks on a clash.

use super::*;

#[test]
fn test_handle_move_rename_success() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");
    fs::create_dir(&left_dir).expect("test setup");
    fs::create_dir(&right_dir).expect("test setup");

    let file_name = "move_ok.txt";
    let source_file = left_dir.join(file_name);
    fs::write(&source_file, b"content").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(left_dir).expect("test setup");
    app.right_panel = Panel::new(right_dir.clone()).expect("test setup");

    assert!(app.left_panel.select_named(file_name));

    handle_move(&mut app).expect("test setup");

    assert!(!source_file.exists());
    let moved_file = right_dir.join(file_name);
    assert!(moved_file.exists());
    assert_eq!(
        fs::read_to_string(moved_file).expect("test setup"),
        "content"
    );
    assert!(app.status_text().contains("Moved to"));

    Ok(())
}

#[test]
fn test_handle_move_directory_rename_success() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");
    fs::create_dir(&left_dir).expect("test setup");
    fs::create_dir(&right_dir).expect("test setup");

    let source_dir = left_dir.join("dir_move");
    fs::create_dir(&source_dir).expect("test setup");
    fs::write(source_dir.join("file.txt"), b"dircontent").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(left_dir).expect("test setup");
    app.right_panel = Panel::new(right_dir.clone()).expect("test setup");

    assert!(app.left_panel.select_named("dir_move"));

    handle_move(&mut app).expect("test setup");

    assert!(!source_dir.exists());
    let moved_dir = right_dir.join("dir_move");
    assert!(moved_dir.exists());
    assert_eq!(
        fs::read_to_string(moved_dir.join("file.txt")).expect("test setup"),
        "dircontent"
    );
    assert!(app.status_text().contains("Moved to"));

    Ok(())
}

#[test]
fn test_handle_move_directory_with_conflict_renames() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");
    fs::create_dir(&left_dir).expect("test setup");
    fs::create_dir(&right_dir).expect("test setup");

    let source_dir = left_dir.join("dir_move");
    fs::create_dir(&source_dir).expect("test setup");
    fs::write(source_dir.join("file.txt"), b"dircontent").expect("test setup");

    fs::create_dir(right_dir.join("dir_move")).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(left_dir).expect("test setup");
    app.right_panel = Panel::new(right_dir.clone()).expect("test setup");

    assert!(app.left_panel.select_named("dir_move"));

    handle_move(&mut app).expect("test setup");

    assert!(!source_dir.exists());
    assert!(right_dir.join("dir_move").exists());
    let moved_dir = right_dir.join("dir_move_1");
    assert!(moved_dir.exists());
    assert_eq!(
        fs::read_to_string(moved_dir.join("file.txt")).expect("test setup"),
        "dircontent"
    );
    assert!(app.status_text().contains("Moved to"));

    Ok(())
}

#[test]
fn test_handle_move_file_with_conflict_renames() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");
    fs::create_dir(&left_dir).expect("test setup");
    fs::create_dir(&right_dir).expect("test setup");

    let file_name = "conflict.txt";
    let source_file = left_dir.join(file_name);
    fs::write(&source_file, b"source").expect("test setup");
    fs::write(right_dir.join(file_name), b"existing").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(left_dir).expect("test setup");
    app.right_panel = Panel::new(right_dir.clone()).expect("test setup");

    assert!(app.left_panel.select_named(file_name));

    handle_move(&mut app).expect("test setup");

    assert!(!source_file.exists());
    assert_eq!(
        fs::read_to_string(right_dir.join(file_name)).expect("test setup"),
        "existing"
    );
    let moved_file = right_dir.join("conflict_1.txt");
    assert!(moved_file.exists());
    assert_eq!(
        fs::read_to_string(moved_file).expect("test setup"),
        "source"
    );
    assert!(app.status_text().contains("Moved to"));

    Ok(())
}
