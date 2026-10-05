//! Purpose: unit tests for `Panel::enter` — files, `..`, vanished and unreadable directories.

use super::*;

/// enter("..") from root: `prev_dir_name` is None, fallback selects first entry
#[test]
fn test_enter_parent_from_root_fallback() -> io::Result<()> {
    let root = if cfg!(windows) { "C:\\" } else { "/" };
    let mut panel = Panel::new(PathBuf::from(root))?;

    // ".." should be the first entry (or root has no "..")
    if let Some(dotdot_pos) = panel.entries.iter().position(|e| e.name == "..") {
        panel.select(Some(dotdot_pos));
        panel.enter()?;
        // Should still be at root with a valid selection
        assert!(panel.selected().is_some());
    }

    Ok(())
}

/// `enter()` on a file entry is a no-op (returns Ok without changing path)
#[test]
fn test_enter_file_is_noop() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("file.txt"), "data")?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    let original_path = panel.path.clone();

    // Select the file
    for (i, entry) in panel.entries.iter().enumerate() {
        if entry.name == "file.txt" {
            panel.select(Some(i));
            break;
        }
    }

    panel.enter()?;
    assert_eq!(panel.path, original_path, "path should not change for file");

    Ok(())
}

/// `enter()` with no selection is a no-op
#[test]
fn test_enter_no_selection_is_noop() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    panel.select(None);
    let original_path = panel.path.clone();

    panel.enter()?;
    assert_eq!(panel.path, original_path);

    Ok(())
}

/// `enter()` into a subdirectory clears marks and selects first entry
#[test]
fn test_enter_subdir_clears_marks() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let subdir = temp_dir.path().join("sub");
    fs::create_dir(&subdir)?;
    fs::write(temp_dir.path().join("file.txt"), "data")?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    panel.marked.insert(FileName::from("file.txt"));

    // Select the subdirectory
    for (i, entry) in panel.entries.iter().enumerate() {
        if entry.name == "sub" && entry.kind.is_dir() {
            panel.select(Some(i));
            break;
        }
    }

    panel.enter()?;
    assert!(panel.marked.is_empty(), "marks should be cleared");
    assert!(panel.path.ends_with("sub"));

    Ok(())
}

/// enter("..") fallback when previous dir name not found in parent entries
#[test]
fn test_enter_parent_prev_dir_deleted_falls_back_to_first() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let child = temp_dir.path().join("child");
    fs::create_dir(&child)?;
    let mut panel = Panel::new(child.clone())?;

    // Delete the child dir so it won't appear when we go up
    fs::remove_dir(&child)?;

    // Select ".."
    if let Some(pos) = panel.entries.iter().position(|e| e.name == "..") {
        panel.select(Some(pos));
        panel.enter()?;
        // Should fall back to selecting first entry (line 223)
        assert_eq!(panel.selected(), Some(0));
    }
    Ok(())
}

/// `enter()` used to commit the path *before* the fallible read, stranding the
/// panel on a directory it had no listing for (unreadable dir, removed dir).
#[test]
fn test_enter_vanished_dir_leaves_panel_untouched() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("a.txt"), "a")?;
    let gone = temp_dir.path().join("gone");
    fs::create_dir(&gone)?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    panel.marked.insert(FileName::from("a.txt"));
    let index = panel
        .entries
        .iter()
        .position(|entry| entry.name == "gone")
        .expect("test setup");
    panel.select(Some(index));

    // Removed after the listing was taken: the entry is still on screen.
    fs::remove_dir(&gone)?;
    let result = panel.enter();

    assert!(result.is_err(), "entering a missing directory must fail");
    assert_eq!(panel.path, temp_dir.path(), "panel must not move");
    assert!(panel.entries.iter().any(|entry| entry.name == "a.txt"));
    assert!(
        panel.entries.iter().all(|entry| entry.name != "gone"),
        "the listing is reloaded, so the removed entry is gone"
    );
    let selected = panel.selected().expect("selection must stay valid");
    assert!(selected < panel.entries.len());
    assert!(panel.marked.contains("a.txt"));

    Ok(())
}

/// Same guarantee via the permission path (skipped when running as root, which
/// ignores the mode bits).
#[test]
#[cfg(unix)]
fn test_enter_unreadable_dir_leaves_panel_untouched() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let locked = temp_dir.path().join("locked");
    fs::create_dir(&locked)?;
    // Listing the parent only needs search permission, so the entry shows up as
    // a directory and read_dir on it then fails with EACCES.
    let _locked = crate::test_support::ModeGuard::set(&locked, 0o000)?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    let index = panel
        .entries
        .iter()
        .position(|entry| entry.name == "locked")
        .expect("test setup");
    panel.select(Some(index));

    let result = panel.enter();

    if result.is_ok() {
        return Ok(()); // root bypasses the mode bits — nothing to assert
    }
    assert_eq!(panel.path, temp_dir.path());
    assert_eq!(panel.selected(), Some(index));

    Ok(())
}
