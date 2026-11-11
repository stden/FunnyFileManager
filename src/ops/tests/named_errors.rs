//! Unit tests: an error says which file or folder it happened at — F3, Enter
//! and `cd` used to say only "Error: Permission denied (os error 13)", and a
//! copy named its source when the destination refused.

use std::fs;
use std::path::Path;

use tempfile::TempDir;

use crate::copy::CopyPrompt;
use crate::models::{App, FileName, Panel};
use crate::status::Status;

fn app_on(folder: &Path) -> App {
    let panel = || Panel::new(folder.to_path_buf()).expect("test setup");
    App::with_panels(panel(), panel())
}

/// The path an `ErrorAt` status names, or a failed batch's first entry.
fn named(status: &Status) -> Option<&Path> {
    match status {
        Status::ErrorAt(failed)
        | Status::CopiedWithErrors {
            first_error: failed,
            ..
        } => Some(failed.path()),
        _ => None,
    }
}

#[cfg(unix)]
#[test]
fn f3_on_a_file_it_cannot_read_names_the_file() {
    let folder = TempDir::new().expect("test setup");
    let link = folder.path().join("gone.txt");
    std::os::unix::fs::symlink("nowhere", &link).expect("test setup");
    let mut app = app_on(folder.path());

    let error = crate::ops::view_path(&mut app, link.clone()).expect_err("unreadable");

    assert_eq!(named(&Status::of_error(&error)), Some(link.as_path()));
}

#[test]
fn enter_on_a_folder_it_cannot_read_names_the_folder() {
    let folder = TempDir::new().expect("test setup");
    let gone = folder.path().join("gone");
    fs::create_dir(&gone).expect("test setup");
    let mut app = app_on(folder.path());
    app.get_active_panel().select_named("gone");
    fs::remove_dir(&gone).expect("test setup");

    let error = app.get_active_panel().enter().expect_err("gone");

    assert_eq!(named(&Status::of_error(&error)), Some(gone.as_path()));
    let shown = Status::of_error(&error).to_string();
    assert!(
        shown.starts_with("Error: ") && shown.contains("gone: "),
        "{shown}"
    );
}

#[test]
fn cd_to_a_folder_it_cannot_reach_names_the_folder() {
    let folder = TempDir::new().expect("test setup");
    let mut app = app_on(folder.path());
    let missing = folder.path().join("missing");

    let command = format!("cd {}", missing.display());
    let error = crate::ops::execute_command(&mut app, &command).expect_err("missing");

    assert_eq!(named(&Status::of_error(&error)), Some(missing.as_path()));
}

/// The destination refused, so the destination is named, not the source.
#[test]
fn a_copy_into_a_folder_that_cannot_be_made_names_that_folder() {
    let folder = TempDir::new().expect("test setup");
    fs::write(folder.path().join("a.txt"), b"a").expect("test setup");
    let blocker = folder.path().join("blocker");
    fs::write(&blocker, b"a file, not a folder").expect("test setup");
    let mut app = app_on(folder.path());
    app.get_active_panel().marked.insert(FileName::new("a.txt"));
    let dialog = CopyPrompt::new("a.txt", &blocker.join("inside/"));

    crate::ops::file::copy_with(&mut app, &dialog);

    assert_eq!(
        named(&app.status),
        Some(blocker.join("inside").as_path()),
        "{}",
        app.status
    );
}

/// The guest's case: a read-only destination folder refuses the new file,
/// and the status names that file there, not the source. Root may write
/// anywhere, so it skips.
#[cfg(unix)]
#[test]
fn a_copy_a_read_only_folder_refuses_names_the_destination() {
    use std::os::unix::fs::PermissionsExt;

    if crate::platform::is_root() {
        return;
    }
    let folder = TempDir::new().expect("test setup");
    fs::write(folder.path().join("a.txt"), b"a").expect("test setup");
    let locked = folder.path().join("locked");
    fs::create_dir(&locked).expect("test setup");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o555)).expect("test setup");
    let mut app = app_on(folder.path());
    app.get_active_panel().marked.insert(FileName::new("a.txt"));

    crate::ops::file::copy_with(&mut app, &CopyPrompt::new("a.txt", &locked));
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).expect("cleanup");

    assert_eq!(
        named(&app.status),
        Some(locked.join("a.txt").as_path()),
        "{}",
        app.status
    );
}
