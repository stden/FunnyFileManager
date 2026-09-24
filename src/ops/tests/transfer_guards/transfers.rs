//! Purpose: F5/F6 refuse a folder into itself, keep going past a failure, and leave no half-written copy.

use super::*;

// -----------------------------------------------------------------------
// A directory must never be copied/moved into its own subtree
// -----------------------------------------------------------------------

/// Regression: with the right panel inside `docs`, F5 on `docs` used to make the
/// recursion descend into the copy it was writing — hundreds of nested levels
/// and duplicated files inside the user's own source directory.
#[test]
fn handle_copy_refuses_directory_into_its_own_child() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let source = temp_dir.path().join("docs");
    let inside = source.join("sub");
    fs::create_dir_all(&inside)?;
    fs::write(source.join("a.txt"), b"a")?;
    fs::write(source.join("b.txt"), b"b")?;

    let mut app = app_with(temp_dir.path(), &inside, "docs")?;
    handle_copy(&mut app)?;

    assert!(!inside.join("docs").exists(), "must not copy into itself");
    assert_eq!(fs::read_dir(&source)?.count(), 3);
    assert!(
        app.status_text().contains("1 error(s)"),
        "status was {:?}",
        app.status_text()
    );
    Ok(())
}

/// Renaming a directory into its own subtree fails with `EINVAL`, which the
/// move fallback treats as a cross-device move — straight into the same
/// runaway recursion.
#[test]
fn handle_move_refuses_directory_into_its_own_child() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let source = temp_dir.path().join("docs");
    let inside = source.join("sub");
    fs::create_dir_all(&inside)?;
    fs::write(source.join("a.txt"), b"a")?;

    let mut app = app_with(temp_dir.path(), &inside, "docs")?;
    handle_move(&mut app)?;

    assert!(source.exists(), "the source must survive a refused move");
    assert!(source.join("a.txt").exists());
    assert!(!inside.join("docs").exists());
    assert!(
        app.status_text().contains("1 error(s)"),
        "status was {:?}",
        app.status_text()
    );
    Ok(())
}

// -----------------------------------------------------------------------
// One failing item must not abandon the rest of the batch
// -----------------------------------------------------------------------

/// Marks are bare names in a `BTreeSet`, so `a_missing.bin` is processed first:
/// the old `?` aborted the loop there and copied nothing at all.
#[test]
fn handle_copy_continues_past_a_failing_item() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let left = temp_dir.path().join("left");
    let right = temp_dir.path().join("right");
    fs::create_dir(&left)?;
    fs::create_dir(&right)?;
    fs::write(left.join("b.txt"), b"b")?;
    fs::write(left.join("c.txt"), b"c")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(left)?;
    app.right_panel = Panel::new(right.clone())?;
    for name in ["a_missing.bin", "b.txt", "c.txt"] {
        app.left_panel.marked.insert(FileName::from(name));
    }

    handle_copy(&mut app)?;

    assert!(right.join("b.txt").exists(), "b.txt after the failure");
    assert!(right.join("c.txt").exists(), "c.txt after the failure");
    assert!(
        app.status_text().contains("Copied 2 item(s), 1 error(s)"),
        "status was {:?}",
        app.status_text()
    );
    assert!(app.left_panel.marked.is_empty(), "marks must be cleared");
    assert!(
        app.right_panel.entries.iter().any(|e| e.name == "b.txt"),
        "panels must be refreshed even when an item failed"
    );
    Ok(())
}

#[test]
fn handle_move_continues_past_a_failing_item() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let left = temp_dir.path().join("left");
    let right = temp_dir.path().join("right");
    fs::create_dir(&left)?;
    fs::create_dir(&right)?;
    fs::write(left.join("b.txt"), b"b")?;
    fs::write(left.join("c.txt"), b"c")?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(left.clone())?;
    app.right_panel = Panel::new(right.clone())?;
    for name in ["a_missing.bin", "b.txt", "c.txt"] {
        app.left_panel.marked.insert(FileName::from(name));
    }

    handle_move(&mut app)?;

    assert!(!left.join("b.txt").exists());
    assert!(right.join("b.txt").exists());
    assert!(right.join("c.txt").exists());
    assert!(
        app.status_text().contains("Moved 2 item(s), 1 error(s)"),
        "status was {:?}",
        app.status_text()
    );
    assert!(app.left_panel.marked.is_empty(), "marks must be cleared");
    Ok(())
}

/// A copy that fails halfway must not leave a partial directory under the final
/// name — it is indistinguishable from a complete copy. A Unix socket is a
/// portable way to make `fs::copy` fail mid-recursion (`open` returns `ENXIO`).
#[cfg(unix)]
#[test]
fn handle_copy_removes_a_half_written_destination() -> io::Result<()> {
    use std::os::unix::net::UnixListener;

    let temp_dir = TempDir::new()?;
    let left = temp_dir.path().join("left");
    let right = temp_dir.path().join("right");
    fs::create_dir(&left)?;
    fs::create_dir(&right)?;
    let source = left.join("project");
    fs::create_dir(&source)?;
    fs::write(source.join("keep.txt"), b"keep")?;
    let _socket = UnixListener::bind(source.join("daemon.sock")).expect("test setup");

    let mut app = app_with(&left, &right, "project")?;
    handle_copy(&mut app)?;

    assert!(
        !right.join("project").exists(),
        "a half-written copy must not survive under the final name"
    );
    assert!(
        app.status_text().contains("1 error(s)"),
        "status was {:?}",
        app.status_text()
    );
    Ok(())
}
