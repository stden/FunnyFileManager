//! Purpose: The view, copy, move, make-folder and delete handlers.

use super::*;

#[test]
fn test_handle_view_text_file() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.txt");
    File::create(&file_path)?.write_all(b"Hello World")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Select the file
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "test.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_view(&mut app)?;

    assert!(app.get_viewer().is_some());
    assert_eq!(app.get_viewer().unwrap().title(), "test.txt");
    assert!(app.get_viewer().unwrap().lines[0].contains("Hello World"));

    Ok(())
}

#[test]
fn test_handle_copy_file() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");
    fs::create_dir(&left_dir)?;
    fs::create_dir(&right_dir)?;

    let source_file = left_dir.join("source.txt");
    File::create(&source_file)?.write_all(b"content")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(left_dir)?;
    app.right_panel = Panel::new(right_dir.clone())?;

    // Select the file
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "source.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_copy(&mut app)?;

    assert!(right_dir.join("source.txt").exists());
    assert!(app.status_text().contains("Copied to"));

    Ok(())
}

#[test]
fn test_handle_move_file() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");
    fs::create_dir(&left_dir)?;
    fs::create_dir(&right_dir)?;

    let source_file = left_dir.join("moveme.txt");
    File::create(&source_file)?.write_all(b"content")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(left_dir.clone())?;
    app.right_panel = Panel::new(right_dir.clone())?;

    // Select the file
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "moveme.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_move(&mut app)?;

    assert!(!left_dir.join("moveme.txt").exists());
    assert!(right_dir.join("moveme.txt").exists());
    assert!(app.status_text().contains("Moved to"));

    Ok(())
}

#[test]
fn test_handle_mkdir_creates_folder() -> io::Result<()> {
    let temp_dir = TempDir::new()?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    handle_mkdir(&mut app)?;

    assert!(temp_dir.path().join("new_folder").exists());
    assert!(app.status_text().contains("Created directory"));

    Ok(())
}

#[test]
fn test_handle_delete_file() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("delete_me.txt");
    File::create(&file_path)?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Select the file
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "delete_me.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_delete_confirm(&mut app)?;
    handle_delete(&mut app)?;

    assert!(!file_path.exists());
    assert!(app.status_text().contains("Deleted"));

    Ok(())
}
