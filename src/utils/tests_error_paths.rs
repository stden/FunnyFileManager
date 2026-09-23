//! Unit tests: what `copy_dir_all`, `copy_symlink` and `refresh_panels`
//! actually do when a step fails part-way through.

use super::tests::copy_step_mutex;

use super::*;

use crate::models::Panel;

use std::fs::File;

use tempfile::TempDir;

fn app_with_panels(left: &Path, right: &Path) -> App {
    App::with_panels(
        Panel::new(left.to_path_buf()).expect("test setup"),
        Panel::new(right.to_path_buf()).expect("test setup"),
    )
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

/// Both panels are reloaded whichever of them lost its folder. The right one,
/// whose folder was deleted, moves to the folder above — it used to fail with
/// "No such file or directory" and keep listing the deleted entries (B27).
#[test]
fn refresh_panels_reloads_the_left_panel_and_moves_the_right_out_of_its_deleted_folder() {
    let _lock = refresh_panels_mutex().lock().expect("refresh mutex");

    let temp_dir = TempDir::new().expect("test setup");
    let left = temp_dir.path().join("left");
    let right = temp_dir.path().join("right");
    fs::create_dir(&left).expect("test setup");
    fs::create_dir(&right).expect("test setup");

    let mut app = app_with_panels(&left, &right);

    // Created after the panels were built: it can only show up if the left
    // panel was actually re-read.
    File::create(left.join("added.txt")).expect("test setup");
    fs::remove_dir(&right).expect("test setup");

    refresh_panels(&mut app).expect("a deleted folder is left, not reported");

    assert!(
        app.left_panel
            .entries
            .iter()
            .any(|entry| entry.name == "added.txt"),
        "the left panel was reloaded"
    );
    assert_eq!(app.right_panel.path, temp_dir.path());
}

/// A left panel whose folder vanished must not cost the right panel its
/// refresh.
///
/// The left reload used to short-circuit with `?`, so deleting a directory the
/// *other* panel was sitting in left the acting panel still listing the entry it
/// had just removed. Both panels are reloaded, and the left one moves out of
/// the deleted folder.
#[test]
fn refresh_panels_reloads_the_right_panel_and_moves_the_left_out_of_its_deleted_folder() {
    let _lock = refresh_panels_mutex().lock().expect("refresh mutex");

    let temp_dir = TempDir::new().expect("test setup");
    let left = temp_dir.path().join("left");
    let right = temp_dir.path().join("right");
    fs::create_dir(&left).expect("test setup");
    fs::create_dir(&right).expect("test setup");

    let mut app = app_with_panels(&left, &right);

    File::create(right.join("added.txt")).expect("test setup");
    fs::remove_dir(&left).expect("test setup");

    refresh_panels(&mut app).expect("a deleted folder is left, not reported");

    assert!(
        app.right_panel
            .entries
            .iter()
            .any(|entry| entry.name == "added.txt"),
        "the right panel was reloaded"
    );
    assert_eq!(app.left_panel.path, temp_dir.path());
}

/// A panel whose folder vanished is never blanked down to a lone "..": it
/// shows the folder above. (A reload that fails for any other reason keeps
/// its last listing — `load_entries` is pinned by
/// `models::tests_panel_symlinks::a_failed_reload_keeps_the_previous_listing`.)
#[test]
fn a_panel_whose_folder_vanished_lists_the_folder_above() {
    let _lock = refresh_panels_mutex().lock().expect("refresh mutex");

    let temp_dir = TempDir::new().expect("test setup");
    let left = temp_dir.path().join("left");
    let right = temp_dir.path().join("right");
    fs::create_dir(&left).expect("test setup");
    fs::create_dir(&right).expect("test setup");
    File::create(left.join("kept.txt")).expect("test setup");

    let mut app = app_with_panels(&left, &right);
    fs::remove_file(left.join("kept.txt")).expect("test setup");
    fs::remove_dir(&left).expect("test setup");

    refresh_panels(&mut app).expect("a deleted folder is left, not reported");

    let names: Vec<&str> = app
        .left_panel
        .entries
        .iter()
        .map(|entry| entry.name.as_str())
        .collect();
    assert!(
        names.contains(&"right"),
        "the folder above is listed: {names:?}"
    );
    assert!(!names.contains(&"kept.txt"));
}
