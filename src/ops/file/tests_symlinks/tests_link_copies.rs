//! Purpose: Copying a top-level link recreates the link, and a copy with no free name fails cleanly.

use super::*;

#[test]
fn copy_recreates_a_top_level_directory_symlink() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let left = temp_dir.path().join("left");
    let right = temp_dir.path().join("right");
    fs::create_dir(&left)?;
    fs::create_dir(&right)?;
    let target = make_target_tree(temp_dir.path())?;
    symlink(&target, left.join("logs"))?;

    let mut app = app_with_panels(&left, &right)?;
    select(&mut app, "logs");
    handle_copy(&mut app)?;

    let copied = right.join("logs");
    assert!(
        fs::symlink_metadata(&copied)?.is_symlink(),
        "F5 must recreate the link, not copy the tree it points at"
    );
    assert_eq!(fs::read_link(&copied)?, target);
    assert!(
        !copied.join("inside.txt").is_file() || fs::read_link(&copied).is_ok(),
        "nothing may have been written through the link"
    );
    Ok(())
}

#[test]
fn copy_recreates_a_top_level_file_symlink() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let left = temp_dir.path().join("left");
    let right = temp_dir.path().join("right");
    fs::create_dir(&left)?;
    fs::create_dir(&right)?;
    fs::write(temp_dir.path().join("real.txt"), b"payload")?;
    symlink(temp_dir.path().join("real.txt"), left.join("alias.txt"))?;

    let mut app = app_with_panels(&left, &right)?;
    select(&mut app, "alias.txt");
    handle_copy(&mut app)?;

    assert!(
        fs::symlink_metadata(right.join("alias.txt"))?.is_symlink(),
        "a file link must survive F5 as a link"
    );
    Ok(())
}

/// A broken link has no target to dereference at all: the old `fs::copy` path
/// failed outright, so the entry could not be transferred.
#[test]
fn copy_recreates_a_broken_symlink() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let left = temp_dir.path().join("left");
    let right = temp_dir.path().join("right");
    fs::create_dir(&left)?;
    fs::create_dir(&right)?;
    symlink("../nowhere/gone.txt", left.join("dangling"))?;

    let mut app = app_with_panels(&left, &right)?;
    select(&mut app, "dangling");
    handle_copy(&mut app)?;

    assert!(fs::symlink_metadata(right.join("dangling"))?.is_symlink());
    assert!(
        app.status_text().starts_with("Copied"),
        "the copy must succeed, got {:?}",
        app.status_text()
    );
    Ok(())
}

/// When every candidate destination name is already taken, the copy fails with
/// `AlreadyExists` rather than overwriting or looping forever.
#[test]
fn copy_fails_cleanly_when_no_unique_destination_remains() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let left = temp_dir.path().join("left");
    let right = temp_dir.path().join("right");
    fs::create_dir(&left)?;
    fs::create_dir(&right)?;
    fs::write(left.join("report.txt"), b"src")?;

    // Occupy the free name and the first 9999 suffixes unique_destination tries.
    fs::write(right.join("report.txt"), b"taken")?;
    for i in 1..=9999u32 {
        fs::write(right.join(format!("report_{i}.txt")), b"taken")?;
    }

    let mut app = app_with_panels(&left, &right)?;
    select(&mut app, "report.txt");
    // Batch copy records per-item failures in the status, not as Err — so the
    // destination still must not have grown a 10000th copy.
    handle_copy(&mut app)?;

    assert!(
        !right.join("report_10000.txt").exists(),
        "must not invent names past the unique-destination limit"
    );
    assert!(
        app.status_text().contains("error")
            || app.status_text().contains("Error")
            || app.status_text().contains("Already")
            || app.status_text().contains("could not"),
        "status should report the failure, got {:?}",
        app.status_text()
    );
    Ok(())
}
