//! Purpose: Copy and move of directories and marked files, with the rename fallback.

use super::*;

// -----------------------------------------------------------------------
// handle_copy / handle_move with directories
// -----------------------------------------------------------------------

#[test]
fn test_handle_copy_directory_recursive() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let src_dir = temp_dir.path().join("source");
    let target = temp_dir.path().join("target");
    fs::create_dir(&src_dir)?;
    fs::create_dir(&target)?;
    fs::write(src_dir.join("inside.txt"), "data")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(target)?;

    // Select the source directory
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "source" && entry.kind.is_dir() {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_copy(&mut app)?;
    // Verify the directory was copied to the target
    assert!(app.right_panel.path.join("source").exists());
    assert!(app
        .right_panel
        .path
        .join("source")
        .join("inside.txt")
        .exists());

    Ok(())
}

#[test]
fn test_handle_move_with_rename_fallback() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let target = temp_dir.path().join("target");
    fs::create_dir(&target)?;
    fs::write(temp_dir.path().join("moveme.txt"), "data")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(target.clone())?;

    // Select the file
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "moveme.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    // Force rename failure to exercise copy+delete fallback
    let guard = ForceRenameFailureGuard::enable();
    let result = handle_move(&mut app);
    drop(guard);

    result?;
    assert!(target.join("moveme.txt").exists());
    assert!(!temp_dir.path().join("moveme.txt").exists());

    Ok(())
}

#[test]
fn test_handle_move_dir_with_rename_fallback() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let src_dir = temp_dir.path().join("movedir");
    let target = temp_dir.path().join("target");
    fs::create_dir(&src_dir)?;
    fs::create_dir(&target)?;
    fs::write(src_dir.join("child.txt"), "data")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(target.clone())?;

    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "movedir" && entry.kind.is_dir() {
            app.left_panel.select(Some(i));
            break;
        }
    }

    let guard = ForceRenameFailureGuard::enable();
    let result = handle_move(&mut app);
    drop(guard);

    result?;
    assert!(target.join("movedir").join("child.txt").exists());
    assert!(!src_dir.exists());

    Ok(())
}

// -----------------------------------------------------------------------
// handle_copy / handle_move with multiple marked files
// -----------------------------------------------------------------------

#[test]
fn test_handle_copy_multiple_marked() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let target = temp_dir.path().join("dest");
    fs::create_dir(&target)?;
    fs::write(temp_dir.path().join("a.txt"), "a")?;
    fs::write(temp_dir.path().join("b.txt"), "b")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(target.clone())?;
    app.left_panel.marked.insert(FileName::from("a.txt"));
    app.left_panel.marked.insert(FileName::from("b.txt"));

    handle_copy(&mut app)?;
    assert!(target.join("a.txt").exists());
    assert!(target.join("b.txt").exists());
    assert!(
        app.status_text().contains("2 item(s)"),
        "should report 2 items: {}",
        app.status_text()
    );

    Ok(())
}

#[test]
fn test_handle_move_empty_sources_is_noop() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.left_panel.entries.clear();
    app.left_panel.select(None);

    handle_move(&mut app)?;
    // Should just return Ok

    Ok(())
}
