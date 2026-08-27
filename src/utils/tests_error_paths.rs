//! Unit tests: what `copy_dir_all`, `copy_symlink` and `refresh_panels`
//! actually do when a step fails part-way through.

use super::tests::copy_step_mutex;
use crate::models::Bookmarks;
use crate::models::ViewMode;
use crate::quicksearch::QuickSearch;
use crate::status::Status;

use super::*;

use crate::models::{Overlay, Panel, PanelSide};

use std::fs::File;

use tempfile::TempDir;

fn app_with_panels(left: &Path, right: &Path) -> App {
    App {
        left_panel: Panel::new(left.to_path_buf()).expect("test setup"),
        right_panel: Panel::new(right.to_path_buf()).expect("test setup"),
        active_panel: PanelSide::Left,
        status: Status::Cleared,
        view: ViewMode::default(),
        overlay: Overlay::None,
        bookmarks: Bookmarks::default(),
        lifecycle: crate::models::Lifecycle::default(),
        quick_search: QuickSearch::default(),
        history: crate::history::History::default(),
        console: crate::console::ConsoleLog::default(),
    }
}

#[test]
fn copy_dir_all_reports_not_found_for_a_missing_source() {
    let _lock = copy_step_mutex().lock().expect("copy step mutex");
    // Only the copy step is reset: `reset_forced_errors` would also clear the
    // refresh flag, which a different mutex guards.
    set_copy_dir_all_step(0);

    // Two separate roots: the self-copy guard compares canonicalised
    // ancestors, and a shared parent would trip it before `read_dir` runs.
    let source_root = TempDir::new().expect("test setup");
    let dest_root = TempDir::new().expect("test setup");
    let dest = dest_root.path().join("out");

    let err = copy_dir_all(&source_root.path().join("missing"), &dest)
        .expect_err("a missing source cannot be copied");

    assert_eq!(err.kind(), io::ErrorKind::NotFound);
    assert!(
        !dest.exists() || fs::read_dir(&dest).expect("dest dir").next().is_none(),
        "a failed copy must not leave files behind"
    );
}

/// A relative path that does not exist has no canonicalisable ancestor, so the
/// self-copy guard cannot compare anything. The copy must then fail honestly
/// at `read_dir` instead of being rejected as a copy into itself.
#[test]
fn copy_dir_all_without_canonical_ancestors_fails_at_read_dir() {
    let _lock = copy_step_mutex().lock().expect("copy step mutex");
    // Only the copy step is reset: `reset_forced_errors` would also clear the
    // refresh flag, which a different mutex guards.
    set_copy_dir_all_step(0);

    let dest_root = TempDir::new().expect("test setup");

    let err = copy_dir_all(
        Path::new("ffm-no-such-relative-source"),
        &dest_root.path().join("out"),
    )
    .expect_err("a missing source cannot be copied");

    assert_eq!(err.kind(), io::ErrorKind::NotFound);
}

/// Copying must never silently replace an existing entry: a destination name
/// already taken makes the symlink step fail and leaves the entry alone.
#[cfg(unix)]
#[test]
fn copy_dir_all_fails_when_a_symlink_name_is_already_taken() -> io::Result<()> {
    use std::os::unix::fs::symlink;

    let _lock = copy_step_mutex().lock().expect("copy step mutex");
    // Only the copy step is reset: `reset_forced_errors` would also clear the
    // refresh flag, which a different mutex guards.
    set_copy_dir_all_step(0);

    let temp_dir = TempDir::new().expect("test setup");
    let src = temp_dir.path().join("src");
    let dst = temp_dir.path().join("dst");
    fs::create_dir(&src).expect("test setup");
    fs::write(src.join("target.txt"), b"target").expect("test setup");
    symlink("target.txt", src.join("link")).expect("test setup");

    fs::create_dir(&dst).expect("test setup");
    fs::write(dst.join("link"), b"existing").expect("test setup");

    let err = copy_dir_all(&src, &dst).expect_err("an occupied name must not be overwritten");

    assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
    assert_eq!(fs::read(dst.join("link"))?, b"existing");
    Ok(())
}

/// `copy_symlink` reads the link target first. Handed something that is not a
/// link, it must report that failure and create nothing.
#[test]
fn copy_symlink_reports_a_read_link_failure_and_creates_nothing() {
    let temp_dir = TempDir::new().expect("test setup");
    let regular = temp_dir.path().join("regular.txt");
    fs::write(&regular, b"not a link").expect("test setup");
    let dest = temp_dir.path().join("copy.txt");

    copy_symlink(&regular, &dest).expect_err("a regular file has no link target");

    assert!(
        !dest.exists(),
        "nothing may be created when the target cannot be read"
    );
}

/// The left panel is reloaded first, so a right-panel failure still leaves the
/// left panel refreshed.
#[test]
fn refresh_panels_reports_the_right_panel_failure_after_reloading_the_left() {
    let _lock = refresh_panels_mutex().lock().expect("refresh mutex");

    let temp_dir = TempDir::new().expect("test setup");
    let left = temp_dir.path().join("left");
    let right = temp_dir.path().join("right");
    fs::create_dir(&left).expect("test setup");
    fs::create_dir(&right).expect("test setup");

    let mut app = app_with_panels(&left, &right);

    // Created after the panels were built: it can only show up if the left
    // panel was actually re-read before the right panel failed.
    File::create(left.join("added.txt")).expect("test setup");
    fs::remove_dir(&right).expect("test setup");

    let err = refresh_panels(&mut app).expect_err("the right panel path no longer exists");

    assert_eq!(err.kind(), io::ErrorKind::NotFound);
    assert!(
        app.left_panel
            .entries
            .iter()
            .any(|entry| entry.name == "added.txt"),
        "the left panel must have been reloaded before the failure"
    );
}

/// A failing left reload must not cost the right panel its refresh.
///
/// The left reload used to short-circuit with `?`, so deleting a directory the
/// *other* panel was sitting in left the acting panel still listing the entry it
/// had just removed. Both panels are reloaded now; the failure is only reported
/// afterwards.
#[test]
fn refresh_panels_reloads_the_right_panel_even_when_the_left_fails() {
    let _lock = refresh_panels_mutex().lock().expect("refresh mutex");

    let temp_dir = TempDir::new().expect("test setup");
    let left = temp_dir.path().join("left");
    let right = temp_dir.path().join("right");
    fs::create_dir(&left).expect("test setup");
    fs::create_dir(&right).expect("test setup");

    let mut app = app_with_panels(&left, &right);

    File::create(right.join("added.txt")).expect("test setup");
    fs::remove_dir(&left).expect("test setup");

    let err = refresh_panels(&mut app).expect_err("the left panel path no longer exists");

    assert_eq!(err.kind(), io::ErrorKind::NotFound);
    assert!(
        app.right_panel
            .entries
            .iter()
            .any(|entry| entry.name == "added.txt"),
        "the right panel must be reloaded even though the left one failed"
    );
}

/// The failing panel keeps its last good listing.
///
/// `load_entries` used to clear `self.entries` and push ".." *before* the
/// fallible `read_dir`, so a panel whose directory vanished was blanked down to
/// a lone ".." instead of continuing to show what it last read successfully.
#[test]
fn a_failed_reload_leaves_the_previous_listing_in_place() {
    let _lock = refresh_panels_mutex().lock().expect("refresh mutex");

    let temp_dir = TempDir::new().expect("test setup");
    let left = temp_dir.path().join("left");
    let right = temp_dir.path().join("right");
    fs::create_dir(&left).expect("test setup");
    fs::create_dir(&right).expect("test setup");
    File::create(left.join("kept.txt")).expect("test setup");

    let mut app = app_with_panels(&left, &right);
    let before = app.left_panel.entries.len();
    assert!(
        app.left_panel
            .entries
            .iter()
            .any(|entry| entry.name == "kept.txt"),
        "test setup: the left panel starts with a real listing"
    );

    fs::remove_file(left.join("kept.txt")).expect("test setup");
    fs::remove_dir(&left).expect("test setup");

    refresh_panels(&mut app).expect_err("the left panel path no longer exists");

    assert_eq!(app.left_panel.entries.len(), before);
    assert!(
        app.left_panel
            .entries
            .iter()
            .any(|entry| entry.name == "kept.txt"),
        "a failed reload must not blank the panel down to a lone \"..\""
    );
}
