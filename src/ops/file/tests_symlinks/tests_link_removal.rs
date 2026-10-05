//! Purpose: A move fallback and F8 remove the link itself and never its target.

use super::*;

/// The move fallback (`rename` fails across devices) is where the old code did
/// real damage: it deep-copied the target tree and then ran `remove_dir_all` on
/// the *link*, because `Path::is_dir` said it was a directory.
#[test]
fn move_fallback_removes_the_link_and_leaves_the_target_alone() -> io::Result<()> {
    let _guard = RENAME_TEST_MUTEX
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    let temp_dir = TempDir::new().expect("test setup");
    let left = temp_dir.path().join("left");
    let right = temp_dir.path().join("right");
    fs::create_dir(&left)?;
    fs::create_dir(&right)?;
    let target = make_target_tree(temp_dir.path())?;
    let link = left.join("logs");
    symlink(&target, &link)?;

    let mut app = app_with_panels(&left, &right)?;
    select(&mut app, "logs");

    FORCE_RENAME_FAILURE.store(true, Ordering::SeqCst);
    let result = handle_move(&mut app);
    FORCE_RENAME_FAILURE.store(false, Ordering::SeqCst);
    result?;

    assert!(
        fs::symlink_metadata(&link).is_err(),
        "the source link must be gone"
    );
    assert!(
        fs::symlink_metadata(right.join("logs"))?.is_symlink(),
        "the destination must be a link, not a copy of the tree"
    );
    assert!(
        target.join("inside.txt").is_file(),
        "the link's target must be untouched by a move of the link"
    );
    Ok(())
}

/// Same rule, third call site: F8 used `Path::is_dir` too, so a directory link
/// was handed to `remove_dir_all`.
#[test]
fn delete_unlinks_a_directory_symlink_without_touching_its_target() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let left = temp_dir.path().join("left");
    let right = temp_dir.path().join("right");
    fs::create_dir(&left)?;
    fs::create_dir(&right)?;
    let target = make_target_tree(temp_dir.path())?;
    let link = left.join("logs");
    symlink(&target, &link)?;

    let mut app = app_with_panels(&left, &right)?;
    select(&mut app, "logs");
    handle_delete_confirm(&mut app)?;
    handle_delete(&mut app)?;

    assert!(
        fs::symlink_metadata(&link).is_err(),
        "the link itself must be removed"
    );
    assert!(
        target.join("inside.txt").is_file(),
        "the link's target must survive F8 on the link"
    );
    assert_eq!(app.status_text(), "Deleted 1 item(s)");
    Ok(())
}

/// Real directories still take the recursive path.
#[test]
fn delete_still_removes_a_real_directory_recursively() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let left = temp_dir.path().join("left");
    let right = temp_dir.path().join("right");
    fs::create_dir(&left)?;
    fs::create_dir(&right)?;
    let nested = left.join("tree");
    fs::create_dir(&nested)?;
    fs::write(nested.join("inside.txt"), b"payload")?;

    let mut app = app_with_panels(&left, &right)?;
    select(&mut app, "tree");
    handle_delete_confirm(&mut app)?;
    handle_delete(&mut app)?;

    assert!(fs::symlink_metadata(&nested).is_err());
    assert_eq!(app.status_text(), "Deleted 1 item(s)");
    Ok(())
}
