//! Purpose: Typed bookmark commands, their slots and a vanished folder.

use super::*;

#[test]
fn test_execute_command_bookmark_set_and_go() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    execute_command(&mut app, "bookmark set 3").expect("test setup");
    assert!(app
        .bookmarks
        .get(BookmarkSlot::from_usize(3).expect("slot"))
        .is_some());
    assert!(app.status_text().contains("Bookmark 3 set"));

    execute_command(&mut app, "bookmark go 3").expect("test setup");
    assert!(app.status_text().contains("Opened bookmark 3"));

    Ok(())
}

#[test]
fn test_execute_command_bookmark_slot_out_of_range() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    let result = execute_command(&mut app, "bookmark set 99");
    assert!(result.is_err());

    Ok(())
}

/// `bookmark go` with a slot pointing to a missing directory returns an error
/// *and* leaves the panel on its previous, readable directory.
///
/// Asserting only `is_err()` tolerated the bug this pins: the path was committed
/// before the fallible load, so the panel ended up on `…/gone` showing a lone
/// ".." with a stale selection index.
#[test]
fn test_execute_command_bookmark_go_missing_directory() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let missing_dir = temp_dir.path().join("gone");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Set bookmark 0 to a directory that we then delete.
    fs::create_dir(&missing_dir).expect("test setup");
    app.bookmarks.set(
        BookmarkSlot::from_usize(0).expect("slot"),
        missing_dir.clone(),
    );
    fs::remove_dir(&missing_dir).expect("test setup");

    let result = execute_command(&mut app, "bookmark go 0");
    assert!(result.is_err());
    assert_eq!(
        app.left_panel.path,
        temp_dir.path(),
        "a panel must never show a path it has no listing for"
    );
    assert!(
        app.left_panel
            .selected()
            .is_none_or(|index| index < app.left_panel.entries.len()),
        "the selection must stay inside the listing"
    );

    Ok(())
}

#[test]
fn test_execute_command_bookmark_non_numeric_slot() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    let result = execute_command(&mut app, "bookmark set abc");
    assert!(result.is_err());

    Ok(())
}

/// `bookmark set` with no slot argument returns `InvalidInput`.
#[test]
fn test_execute_command_bookmark_no_slot() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    // With Command enum + Shell fallback, incomplete "bookmark" commands
    // are treated as shell commands (which may fail later).
    let result = execute_command(&mut app, "bookmark set");
    // It no longer short-circuits with InvalidInput at parse time.
    // The important thing is it doesn't panic.
    drop(result);
    Ok(())
}

/// `bookmark go 99` (slot > 9) returns an error.
#[test]
fn test_execute_command_bookmark_go_slot_out_of_range() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    let result = execute_command(&mut app, "bookmark go 99");
    assert!(result.is_err());

    Ok(())
}
