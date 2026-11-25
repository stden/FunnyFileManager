//! Purpose: unit tests for `refresh_panels` when a panel's folder was deleted under it.

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
fn test_refresh_panels_with_deleted_directories() -> io::Result<()> {
    let _lock = refresh_panels_mutex().lock().expect("refresh mutex");
    let temp_dir = TempDir::new().expect("test setup");
    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");

    fs::create_dir(&left_dir).expect("test setup");
    fs::create_dir(&right_dir).expect("test setup");

    let mut app = App::with_panels(
        Panel::new(left_dir.clone()).expect("test setup"),
        Panel::new(right_dir.clone()).expect("test setup"),
    );

    fs::remove_dir(&left_dir).expect("test setup");
    fs::remove_dir(&right_dir).expect("test setup");

    let result = refresh_panels(&mut app);

    assert!(result.is_err() || app.left_panel.path != left_dir);

    Ok(())
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
