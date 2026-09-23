//! Unit tests: copy move (split out to keep files under 500 lines).

use super::support::*;
use crate::ops::*;

use crate::models::{App, Panel};

use std::fs::{self, File};

use std::io::{self, Write};

use tempfile::TempDir;

#[test]
fn test_handle_copy_directory() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");
    fs::create_dir(&left_dir).expect("test setup");
    fs::create_dir(&right_dir).expect("test setup");

    // Create a directory with contents
    let source_dir = left_dir.join("source_dir");
    fs::create_dir(&source_dir).expect("test setup");
    File::create(source_dir.join("file.txt"))
        .expect("test setup")
        .write_all(b"content")
        .expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(left_dir).expect("test setup");
    app.right_panel = Panel::new(right_dir.clone()).expect("test setup");

    // Select the directory
    assert!(app.left_panel.select_named("source_dir"));

    handle_copy(&mut app).expect("test setup");

    assert!(right_dir.join("source_dir").exists());
    assert!(right_dir.join("source_dir").join("file.txt").exists());
    assert!(app.status_text().contains("Copied to"));

    Ok(())
}

#[test]
fn test_handle_copy_directory_with_conflict() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");
    fs::create_dir(&left_dir).expect("test setup");
    fs::create_dir(&right_dir).expect("test setup");

    let source_dir = left_dir.join("source_dir");
    fs::create_dir(&source_dir).expect("test setup");
    fs::write(source_dir.join("file.txt"), b"content").expect("test setup");

    fs::create_dir(right_dir.join("source_dir")).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(left_dir).expect("test setup");
    app.right_panel = Panel::new(right_dir.clone()).expect("test setup");

    assert!(app.left_panel.select_named("source_dir"));

    handle_copy(&mut app).expect("test setup");

    assert!(right_dir.join("source_dir").exists());
    let copied = right_dir.join("source_dir_1");
    assert!(copied.exists());
    assert_eq!(
        fs::read_to_string(copied.join("file.txt")).expect("test setup"),
        "content"
    );
    assert!(app.status_text().contains("Copied to"));

    Ok(())
}

#[test]
fn test_handle_copy_file() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");
    fs::create_dir(&left_dir).expect("test setup");
    fs::create_dir(&right_dir).expect("test setup");

    let file_name = "alpha.txt";
    let source_file = left_dir.join(file_name);
    fs::write(&source_file, b"alpha").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(left_dir).expect("test setup");
    app.right_panel = Panel::new(right_dir.clone()).expect("test setup");

    assert!(app.left_panel.select_named(file_name));

    handle_copy(&mut app).expect("test setup");

    let copied = right_dir.join(file_name);
    assert!(source_file.exists());
    assert!(copied.exists());
    assert_eq!(fs::read_to_string(copied).expect("test setup"), "alpha");
    assert!(app.status_text().contains("Copied to"));

    Ok(())
}

#[test]
fn test_handle_copy_file_with_conflict() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");
    fs::create_dir(&left_dir).expect("test setup");
    fs::create_dir(&right_dir).expect("test setup");

    let file_name = "data.txt";
    let source_file = left_dir.join(file_name);
    fs::write(&source_file, b"source").expect("test setup");
    fs::write(right_dir.join(file_name), b"existing").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(left_dir).expect("test setup");
    app.right_panel = Panel::new(right_dir.clone()).expect("test setup");

    assert!(app.left_panel.select_named(file_name));

    handle_copy(&mut app).expect("test setup");

    let copied = right_dir.join("data_1.txt");
    assert!(source_file.exists());
    assert!(copied.exists());
    assert_eq!(fs::read_to_string(copied).expect("test setup"), "source");
    assert!(app.status_text().contains("Copied to"));

    Ok(())
}

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

#[test]
fn test_handle_mkdir_multiple() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Create first directory
    handle_mkdir(&mut app).expect("test setup");
    assert!(temp_dir.path().join("new_folder").exists());

    // Create second directory (should be numbered)
    handle_mkdir(&mut app).expect("test setup");
    assert!(temp_dir.path().join("new_folder_1").exists());

    // Create third directory
    handle_mkdir(&mut app).expect("test setup");
    assert!(temp_dir.path().join("new_folder_2").exists());

    Ok(())
}

#[test]
fn test_handle_delete_no_selection() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.left_panel.select(None);

    // Should not crash
    handle_delete(&mut app).expect("test setup");

    Ok(())
}
