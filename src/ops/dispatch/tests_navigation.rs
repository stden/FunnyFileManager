//! Purpose: `:cd` and `:bookmark go` must move the panel the same way every
//! other navigation does — through `Panel::navigate_to`, which binds path,
//! marks and selection together and rolls back when the listing fails.
//!
//! Both commands used to hand-roll `clear_marks(); panel.path = …;
//! load_entries()`, which commits the path *before* the fallible read: a
//! bookmark to a since-deleted directory left the panel on a path it had no
//! listing for, with the marks already dropped.

use super::execute_command;
use crate::models::BookmarkSlot;
use crate::models::FileName;
use crate::models::{App, Panel};

use std::fs;
use tempfile::TempDir;

fn app_at(dir: &TempDir) -> App {
    let mut app = App::new().expect("test app");
    app.left_panel = Panel::new(dir.path().to_path_buf()).expect("panel");
    app.right_panel = Panel::new(dir.path().to_path_buf()).expect("panel");
    app
}

/// A bookmark is only a remembered path: the directory may be gone by the time
/// it is used, and nothing else in `bookmark go` checks that it still exists.
#[test]
fn bookmark_go_to_a_removed_directory_leaves_the_panel_where_it_was() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("keep.txt"), b"x").expect("write");
    let gone = temp.path().join("gone");
    fs::create_dir(&gone).expect("mkdir");

    let mut app = app_at(&temp);
    app.get_active_panel()
        .marked
        .insert(FileName::from("keep.txt"));
    app.bookmarks
        .set(BookmarkSlot::from_usize(0).expect("slot"), gone.clone());
    fs::remove_dir(&gone).expect("rmdir");

    let error = execute_command(&mut app, "bookmark go 0").expect_err("the directory is gone");
    assert_eq!(error.kind(), std::io::ErrorKind::NotFound);

    let panel = app.get_active_panel();
    assert_eq!(
        panel.path,
        temp.path(),
        "a panel must never show a path it has no listing for"
    );
    assert!(
        panel.entries.iter().any(|entry| entry.name == "keep.txt"),
        "the previous listing survives the failed jump"
    );
    assert!(
        panel.marked.contains("keep.txt"),
        "marks belong to the directory we never left"
    );
    assert!(
        panel
            .selected()
            .is_some_and(|index| index < panel.entries.len()),
        "the selection must stay inside the listing"
    );
}

/// `canonicalize` + `is_dir` both succeed for a directory we may traverse but
/// not read (`cd /root` as a normal user); only `read_dir` fails. The panel must
/// not have moved by then.
///
/// Skipped when the listing is readable anyway — as root the mode bits below do
/// not deny anything, so there is nothing to assert.
#[cfg(unix)]
#[test]
fn cd_into_a_traversable_but_unreadable_directory_leaves_the_panel_where_it_was() {
    use std::os::unix::fs::PermissionsExt;

    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("keep.txt"), b"x").expect("write");
    let locked = temp.path().join("locked");
    fs::create_dir(&locked).expect("mkdir");
    // --x--x--x: `cd` and `stat` still work, `read_dir` does not.
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o111)).expect("chmod");

    if fs::read_dir(&locked).is_ok() {
        return; // running with privileges that ignore the mode bits
    }

    let mut app = app_at(&temp);
    app.get_active_panel()
        .marked
        .insert(FileName::from("keep.txt"));

    let error = execute_command(&mut app, "cd locked").expect_err("the listing is denied");
    assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);

    let panel = app.get_active_panel();
    assert_eq!(panel.path, temp.path(), "the panel must not have moved");
    assert!(panel.entries.iter().any(|entry| entry.name == "keep.txt"));
    assert!(panel.marked.contains("keep.txt"));

    // Restore so the TempDir can be removed.
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).expect("chmod");
}
