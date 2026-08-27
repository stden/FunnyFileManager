//! Unit tests: an operation that succeeded must keep saying so, even when the
//! panel reload that follows it fails.
//!
//! `handle_delete` set `Status::Deleted { count }` and then propagated the error
//! from `refresh_panels`, so the caller in `popup.rs` overwrote it with
//! "Delete failed: No such file or directory" — the user is told their data is
//! intact when it is gone. No race is needed: the other panel sitting inside the
//! directory being deleted makes its reload fail every time.

use super::*;

use crate::models::{Overlay, Panel, PanelSide};

use crate::utils::{refresh_panels_mutex, reset_forced_errors, set_refresh_panels_error};

use tempfile::TempDir;

/// Two panels rooted at `left` / `right`, left active — the F5/F6/F7/F8 setup.
fn app_with_panels(left: &Path, right: &Path) -> io::Result<App> {
    let mut app = App::new()?;
    app.left_panel = Panel::new(left.to_path_buf())?;
    app.right_panel = Panel::new(right.to_path_buf())?;
    app.active_panel = PanelSide::Left;
    Ok(app)
}

/// Run `operation` with every `refresh_panels` call failing, then clear the flag
/// *before* the assertions so a failing assert cannot leak it into other tests.
fn with_forced_refresh_failure(
    app: &mut App,
    operation: fn(&mut App) -> io::Result<()>,
) -> io::Result<()> {
    let _lock = refresh_panels_mutex().lock().expect("refresh mutex");
    reset_forced_errors();
    set_refresh_panels_error(true);
    let result = operation(app);
    set_refresh_panels_error(false);
    result
}

/// The verified user-visible case: F8 on a directory the other panel is inside.
#[test]
fn delete_keeps_its_success_status_when_the_other_panel_can_no_longer_be_read() -> io::Result<()> {
    let _lock = refresh_panels_mutex().lock().expect("refresh mutex");
    reset_forced_errors();

    let temp_dir = TempDir::new().expect("test setup");
    let doomed = temp_dir.path().join("sub");
    fs::create_dir(&doomed)?;

    // Left panel (active) lists the parent; the right panel sits *inside* the
    // directory under the cursor, so its reload fails once the delete lands.
    let mut app = app_with_panels(temp_dir.path(), &doomed)?;
    app.set_overlay(Overlay::DeleteConfirm(vec![doomed.clone()]));

    handle_delete(&mut app).expect("a failed refresh must not fail the delete");

    // `symlink_metadata`, not `exists()`: the latter follows links and would
    // report "gone" for a live link whose target vanished.
    assert!(
        fs::symlink_metadata(&doomed).is_err(),
        "the directory really was deleted"
    );
    assert_eq!(
        app.status_text(),
        "Deleted 1 item(s)",
        "the delete succeeded — the stale panel must not be reported as a failure"
    );
    Ok(())
}

#[test]
fn copy_keeps_its_success_status_when_the_refresh_fails() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let left = temp_dir.path().join("left");
    let right = temp_dir.path().join("right");
    fs::create_dir(&left)?;
    fs::create_dir(&right)?;
    fs::write(left.join("notes.txt"), b"payload")?;

    let mut app = app_with_panels(&left, &right)?;
    let index = app
        .left_panel
        .entries
        .iter()
        .position(|entry| entry.name == "notes.txt")
        .expect("test setup: entry must be listed");
    app.left_panel.select(Some(index));

    with_forced_refresh_failure(&mut app, handle_copy).expect("the copy itself succeeded");

    assert_eq!(fs::read(right.join("notes.txt"))?, b"payload");
    assert!(
        app.status_text().starts_with("Copied to "),
        "status was {:?}",
        app.status_text()
    );
    Ok(())
}

#[test]
fn move_keeps_its_success_status_when_the_refresh_fails() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let left = temp_dir.path().join("left");
    let right = temp_dir.path().join("right");
    fs::create_dir(&left)?;
    fs::create_dir(&right)?;
    fs::write(left.join("notes.txt"), b"payload")?;

    let mut app = app_with_panels(&left, &right)?;
    let index = app
        .left_panel
        .entries
        .iter()
        .position(|entry| entry.name == "notes.txt")
        .expect("test setup: entry must be listed");
    app.left_panel.select(Some(index));

    with_forced_refresh_failure(&mut app, handle_move).expect("the move itself succeeded");

    assert_eq!(fs::read(right.join("notes.txt"))?, b"payload");
    assert!(fs::symlink_metadata(left.join("notes.txt")).is_err());
    assert!(
        app.status_text().starts_with("Moved to "),
        "status was {:?}",
        app.status_text()
    );
    Ok(())
}

#[test]
fn mkdir_keeps_its_success_status_when_the_refresh_fails() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = app_with_panels(temp_dir.path(), temp_dir.path())?;

    with_forced_refresh_failure(&mut app, handle_mkdir).expect("the mkdir itself succeeded");

    assert!(temp_dir.path().join("new_folder").is_dir());
    assert!(
        app.status_text().starts_with("Created directory "),
        "status was {:?}",
        app.status_text()
    );
    Ok(())
}

#[test]
fn create_file_keeps_its_success_status_when_the_refresh_fails() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = app_with_panels(temp_dir.path(), temp_dir.path())?;

    let _lock = refresh_panels_mutex().lock().expect("refresh mutex");
    reset_forced_errors();
    set_refresh_panels_error(true);
    let result = handle_create_file(&mut app, "fresh.txt");
    set_refresh_panels_error(false);
    result.expect("the create itself succeeded");

    assert!(temp_dir.path().join("fresh.txt").is_file());
    assert_eq!(app.status_text(), "Created file: fresh.txt");
    Ok(())
}
