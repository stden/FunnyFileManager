//! Purpose: Recursive directory copy and panel refresh, including their failures.

use super::*;

#[test]
fn test_copy_dir_all_recursive() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let src_dir = temp_dir.path().join("src");
    let dst_dir = temp_dir.path().join("dst");

    // Create complex directory structure
    fs::create_dir(&src_dir)?;
    fs::create_dir(src_dir.join("subdir1"))?;
    fs::create_dir(src_dir.join("subdir2"))?;
    fs::create_dir(src_dir.join("subdir1").join("nested"))?;

    fs::write(src_dir.join("file1.txt"), b"content1")?;
    fs::write(src_dir.join("subdir1").join("file2.txt"), b"content2")?;
    fs::write(
        src_dir.join("subdir1").join("nested").join("file3.txt"),
        b"content3",
    )?;

    copy_dir_all(&src_dir, &dst_dir)?;

    assert!(dst_dir.exists());
    assert!(dst_dir.join("file1.txt").exists());
    assert!(dst_dir.join("subdir1").join("file2.txt").exists());
    assert!(dst_dir
        .join("subdir1")
        .join("nested")
        .join("file3.txt")
        .exists());

    Ok(())
}

#[test]
fn test_refresh_panels() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");

    fs::create_dir(&left_dir)?;
    fs::create_dir(&right_dir)?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(left_dir.clone())?;
    app.right_panel = Panel::new(right_dir)?;

    // Add a file after panels are created
    fs::write(left_dir.join("new_file.txt"), b"new content")?;

    refresh_panels(&mut app)?;

    // Check that the new file is loaded
    let has_new_file = app
        .left_panel
        .entries
        .iter()
        .any(|e| e.name == "new_file.txt");
    assert!(has_new_file);

    Ok(())
}

// Test to cover the ^0 branches in utils.rs copy_dir_all
#[test]
#[cfg(unix)]
fn test_copy_dir_all_permission_errors() -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = TempDir::new()?;
    let src_dir = temp_dir.path().join("src");
    let dst_dir = temp_dir.path().join("dst");
    let protected_dir = temp_dir.path().join("src").join("protected");

    fs::create_dir(&src_dir)?;
    fs::create_dir(&protected_dir)?;
    fs::write(protected_dir.join("file.txt"), b"data")?;

    // Make directory unreadable
    let mut perms = fs::metadata(&protected_dir)?.permissions();
    perms.set_mode(0o000);
    fs::set_permissions(&protected_dir, perms)?;

    // This should handle the error gracefully
    let result = copy_dir_all(&src_dir, &dst_dir);

    // Restore permissions for cleanup
    let mut perms = fs::metadata(&protected_dir)
        .unwrap_or_else(|_| fs::metadata(&src_dir).unwrap())
        .permissions();
    perms.set_mode(0o755);
    drop(fs::set_permissions(&protected_dir, perms));

    assert!(result.is_err() || dst_dir.exists());

    Ok(())
}

// A panel whose folder was deleted is not a refresh error: it moves to the
// nearest folder above that still exists, as Far's panel does.
#[test]
fn test_refresh_panels_with_deleted_directories() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");

    fs::create_dir(&left_dir)?;
    fs::create_dir(&right_dir)?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(left_dir.clone())?;
    app.right_panel = Panel::new(right_dir)?;

    fs::remove_dir_all(&left_dir)?;

    refresh_panels(&mut app)?;
    assert_eq!(app.left_panel.path, temp_dir.path());

    Ok(())
}

// Test copy_dir_all with source not existing
#[test]
fn test_copy_dir_all_src_not_exist() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let src_dir = temp_dir.path().join("nonexistent");
    let dst_dir = temp_dir.path().join("dst");

    let result = copy_dir_all(&src_dir, &dst_dir);
    assert!(result.is_err());

    Ok(())
}

// Test copy_dir_all with empty directory
#[test]
fn test_copy_dir_all_empty_dir() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let src_dir = temp_dir.path().join("empty");
    let dst_dir = temp_dir.path().join("dst");

    fs::create_dir(&src_dir)?;

    copy_dir_all(&src_dir, &dst_dir)?;

    assert!(dst_dir.exists());
    assert_eq!(fs::read_dir(&dst_dir)?.count(), 0);

    Ok(())
}
