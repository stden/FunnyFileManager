//! Purpose: F5 of files and folders, with and without a name already taken.

use super::*;

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
