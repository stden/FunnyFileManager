//! Purpose: View, copy, move and make-folder handlers on errors and marked files.

use super::*;

#[test]
#[cfg(unix)]
fn test_handle_view_io_error() -> io::Result<()> {
    use funny_file_manager::handlers::handle_view;
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("unreadable.txt");
    File::create(&file_path)?;

    fs::set_permissions(&file_path, fs::Permissions::from_mode(0o000))?;
    // Root bypasses Unix DAC permissions, so a 0o000 file may still be readable;
    // only assert the IO error when permissions are actually enforced.
    let perms_enforced = fs::read(&file_path).is_err();

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;

    if let Some(i) = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "unreadable.txt")
    {
        app.left_panel.select(Some(i));
        let result = handle_view(&mut app);
        if perms_enforced {
            assert!(result.is_err());
        }
    }

    fs::set_permissions(&file_path, fs::Permissions::from_mode(0o644))?;
    Ok(())
}

#[test]
fn test_handle_copy_marked_files() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");
    fs::create_dir(&left_dir)?;
    fs::create_dir(&right_dir)?;

    File::create(left_dir.join("a.txt"))?;
    File::create(left_dir.join("b.txt"))?;
    File::create(left_dir.join("c.txt"))?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(left_dir)?;
    app.right_panel = Panel::new(right_dir.clone())?;

    // Mark a.txt and c.txt
    app.left_panel.marked.insert(FileName::from("a.txt"));
    app.left_panel.marked.insert(FileName::from("c.txt"));

    handle_copy(&mut app)?;

    assert!(right_dir.join("a.txt").exists());
    assert!(!right_dir.join("b.txt").exists());
    assert!(right_dir.join("c.txt").exists());
    assert!(app.status_text().contains("Copied 2 item(s)"));

    Ok(())
}

#[test]
fn test_handle_move_marked_files() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");
    fs::create_dir(&left_dir)?;
    fs::create_dir(&right_dir)?;

    File::create(left_dir.join("a.txt"))?;
    File::create(left_dir.join("b.txt"))?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(left_dir.clone())?;
    app.right_panel = Panel::new(right_dir.clone())?;

    // Mark a.txt
    app.left_panel.marked.insert(FileName::from("a.txt"));

    handle_move(&mut app)?;

    assert!(!left_dir.join("a.txt").exists());
    assert!(right_dir.join("a.txt").exists());
    assert!(left_dir.join("b.txt").exists());

    Ok(())
}

#[test]
fn test_handle_mkdir_failure() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    // Create a file where we want to create a directory
    let file_path = temp_dir.path().join("blocking_file");
    File::create(&file_path)?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Try to mkdir inside the file path (which is impossible)
    app.left_panel.path = file_path;

    // This should fail
    let result = handle_mkdir(&mut app);
    assert!(result.is_err());

    Ok(())
}
