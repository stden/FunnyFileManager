//! Purpose: The Temp row empties the temporary folder without removing it, and
//! only ever touches what the current user owns there.
//!
//! The temporary folder is shared: `/tmp` holds every user's files and the
//! sockets of running programs, and this session's own `TMPDIR` is where
//! Ctrl+G writes its list files. Each test takes the dialog's real Temp row and
//! points it at a folder inside a `TempDir`.

use std::fs;
use std::path::Path;

use tempfile::TempDir;

use crate::models::{CleanupCandidate, DiskCleanupDialog, FileSize};
use crate::test_support::disk_cleanup::{app, dialog, run};

/// The dialog's own Temp row, aimed at `folder` and ticked.
fn temp_row(folder: &Path) -> CleanupCandidate {
    let row = DiskCleanupDialog::new()
        .candidates
        .into_iter()
        .find(|row| row.label.contains("Temp"))
        .expect("the dialog offers a Temp row");
    CleanupCandidate {
        path: folder.to_path_buf(),
        checked: true,
        exists: true,
        ..row
    }
}

fn names_in(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(folder)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn temp_cleanup_empties_the_temp_folder_and_keeps_the_folder_itself() {
    let temp = TempDir::new().unwrap();
    let folder = temp.path().join("Temp");
    fs::create_dir_all(folder.join("nested")).unwrap();
    fs::write(folder.join("stale.log"), b"abc").unwrap();
    fs::write(folder.join("nested/data.bin"), b"12345").unwrap();
    let mut app = app(temp.path(), vec![temp_row(&folder)]);

    run(&mut app);

    assert!(folder.is_dir(), "the temporary folder itself must survive");
    assert_eq!(names_in(&folder), Vec::<String>::new());
    assert_eq!(app.status.to_string(), "Freed 8 B");
}

#[test]
fn measuring_the_temp_row_counts_its_contents_and_ticks_it() {
    let temp = TempDir::new().unwrap();
    let folder = temp.path().join("Temp");
    fs::create_dir(&folder).unwrap();
    fs::write(folder.join("old.bin"), b"old").unwrap();
    let mut dialog = dialog(vec![temp_row(&folder)]);

    super::super::measure(&mut dialog, temp.path());

    assert_eq!(dialog.candidates[0].size, Some(FileSize::from_bytes(3)));
    assert!(dialog.candidates[0].checked);
}

/// Only root can hand a file to another user, so elsewhere this is skipped.
#[cfg(unix)]
#[test]
fn temp_cleanup_leaves_other_users_files_alone() {
    const SOMEONE_ELSE: u32 = 4_242_424;
    let temp = TempDir::new().unwrap();
    let folder = temp.path().join("Temp");
    fs::create_dir_all(folder.join("mine")).unwrap();
    fs::write(folder.join("mine.txt"), b"mine").unwrap();
    fs::write(folder.join("theirs.txt"), b"theirs").unwrap();
    fs::write(folder.join("mine/theirs.txt"), b"theirs").unwrap();
    for theirs in ["theirs.txt", "mine/theirs.txt"] {
        let path = folder.join(theirs);
        if std::os::unix::fs::chown(&path, Some(SOMEONE_ELSE), Some(SOMEONE_ELSE)).is_err() {
            eprintln!("skipped: only root can give a file to another user");
            return;
        }
    }
    let mut app = app(temp.path(), vec![temp_row(&folder)]);

    run(&mut app);

    assert_eq!(names_in(&folder), ["mine", "theirs.txt"]);
    assert_eq!(names_in(&folder.join("mine")), ["theirs.txt"]);
    assert_eq!(app.status.to_string(), "Freed 4 B");
}

/// A socket in the temporary folder is how a running program (tmux, an SSH
/// agent) is reached; removing it frees nothing and cuts that program off.
#[cfg(unix)]
#[test]
fn temp_cleanup_keeps_the_sockets_of_running_programs() {
    let temp = TempDir::new().unwrap();
    let folder = temp.path().join("Temp");
    fs::create_dir_all(folder.join("tmux-1000")).unwrap();
    let _listener =
        std::os::unix::net::UnixListener::bind(folder.join("tmux-1000/default")).unwrap();
    fs::write(folder.join("stale.log"), b"abc").unwrap();
    let mut app = app(temp.path(), vec![temp_row(&folder)]);

    run(&mut app);

    assert_eq!(names_in(&folder), ["tmux-1000"]);
    assert_eq!(names_in(&folder.join("tmux-1000")), ["default"]);
    assert_eq!(app.status.to_string(), "Freed 3 B");
}
