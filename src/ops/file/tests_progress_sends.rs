//! Purpose: The file a F6 sends is named to the progress dialog with its
//! destination, from the quick transfer and from the F5/F6 dialog alike.

use std::fs;
use std::path::PathBuf;
use std::sync::PoisonError;

use tempfile::TempDir;

use super::tests_progress::{app_on, Recorder};
use super::RENAME_TEST_MUTEX;
use crate::copy::{CopyPrompt, Purpose};

mod tests_regressions;

/// The destination of the last file the dialog was told it sends.
fn last_sent_to(recorder: &Recorder) -> Option<PathBuf> {
    recorder
        .frames()
        .into_iter()
        .filter_map(|frame| frame.transfer)
        .next_back()
        .map(|transfer| transfer.destination)
}

/// `from/a.txt` is on the left, `to/` is on the right; both are in `temp`.
fn one_file_to_move(temp: &TempDir) -> (PathBuf, PathBuf) {
    let (from, to) = (temp.path().join("from"), temp.path().join("to"));
    fs::create_dir_all(&from).expect("test setup");
    fs::create_dir(&to).expect("test setup");
    fs::write(from.join("a.txt"), b"abc").expect("test setup");
    (from, to)
}

#[test]
fn a_quick_move_by_rename_names_the_file_it_sends_and_where() {
    let _serial = RENAME_TEST_MUTEX
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let (recorder, _installed) = Recorder::install();
    let temp = TempDir::new().expect("test setup");
    let (from, to) = one_file_to_move(&temp);
    let mut app = app_on(&from, "a.txt", &to);

    crate::ops::handle_move(&mut app).expect("test setup");

    assert!(to.join("a.txt").exists());
    assert_eq!(last_sent_to(&recorder), Some(to.join("a.txt")));
}

#[test]
fn the_move_dialog_names_the_file_it_sends_and_where() {
    let _serial = RENAME_TEST_MUTEX
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let (recorder, _installed) = Recorder::install();
    let temp = TempDir::new().expect("test setup");
    let (from, to) = one_file_to_move(&temp);
    let mut app = app_on(&from, "a.txt", &to);
    let settings = CopyPrompt::for_purpose(Purpose::Move, "a.txt", &to);

    crate::ops::copy_with(&mut app, &settings);

    assert!(to.join("a.txt").exists());
    assert_eq!(last_sent_to(&recorder), Some(to.join("a.txt")));
}
