//! Purpose: What F5, F6 and F8 tell the progress dialog: a copy and a delete
//! measured first and counted to the end, a move counted by the entries it
//! sends and the bytes it copies across filesystems, and a delete that never
//! follows a link while it reports.

use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::rc::Rc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use tempfile::TempDir;

use super::{FORCE_RENAME_FAILURE, RENAME_TEST_MUTEX};
use crate::models::{App, Panel};
use crate::progress::{
    install_with, is_shown, take_painted, Installed, Operation, Painter, Phase, Snapshot, Tally,
};

/// Every snapshot it was asked to draw.
#[derive(Clone, Default)]
pub(super) struct Recorder(Rc<RefCell<Vec<Snapshot>>>);

impl Painter for Recorder {
    fn paint(&mut self, snapshot: &Snapshot) {
        self.0.borrow_mut().push(snapshot.clone());
    }
}

impl Recorder {
    /// Installed, drawing every report.
    pub(super) fn install() -> (Self, Installed) {
        let recorder = Self::default();
        let installed = install_with(Box::new(recorder.clone()), Duration::ZERO);
        (recorder, installed)
    }

    pub(super) fn frames(&self) -> Vec<Snapshot> {
        self.0.borrow().clone()
    }

    fn last(&self) -> Snapshot {
        self.frames().last().cloned().expect("something was drawn")
    }
}

const fn tally(files: u64, bytes: u64) -> Tally {
    Tally { files, bytes }
}

/// `root/` with `a.txt` (3 bytes) and `sub/b.txt` (5 bytes).
fn tree(root: &Path) {
    fs::create_dir_all(root.join("sub")).expect("test setup");
    fs::write(root.join("a.txt"), b"abc").expect("test setup");
    fs::write(root.join("sub").join("b.txt"), b"hello").expect("test setup");
}

/// An app whose left panel is on `from` with `name` selected, and whose
/// right panel is on `to`.
pub(super) fn app_on(from: &Path, name: &str, to: &Path) -> App {
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(from.to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(to.to_path_buf()).expect("test setup");
    let row = app
        .left_panel
        .entries
        .iter()
        .position(|entry| entry.name == name)
        .expect("test setup");
    app.left_panel.select(Some(row));
    app
}

#[test]
fn a_copy_is_measured_first_then_counted_to_the_end() {
    let (recorder, _installed) = Recorder::install();
    let temp = TempDir::new().expect("test setup");
    let (from, to) = (temp.path().join("from"), temp.path().join("to"));
    tree(&from.join("folder"));
    fs::create_dir(&to).expect("test setup");
    let mut app = app_on(&from, "folder", &to);

    crate::ops::handle_copy(&mut app).expect("test setup");

    assert!(to.join("folder").join("sub").join("b.txt").exists());
    let frames = recorder.frames();
    assert!(
        frames.iter().any(|frame| frame.phase == Phase::Scanning),
        "the sources are measured first"
    );
    let last = recorder.last();
    assert_eq!(last.operation, Operation::Copy);
    assert_eq!(last.total, tally(2, 8));
    assert_eq!(last.done, last.total);
    assert_eq!(last.permille(), 1000);
    assert!(
        frames
            .iter()
            .any(|frame| frame.current.ends_with(Path::new("sub").join("b.txt"))),
        "the dialog names the file being copied"
    );
    assert!(take_painted(), "the loop is told to repaint the screen");
    assert!(!take_painted(), "and told once");
    assert!(!is_shown(), "the operation is no longer shown");
}

#[test]
fn a_delete_counts_every_file_it_removes() {
    let (recorder, _installed) = Recorder::install();
    let temp = TempDir::new().expect("test setup");
    tree(&temp.path().join("folder"));
    let mut app = app_on(temp.path(), "folder", temp.path());

    crate::ops::handle_delete_confirm(&mut app).expect("test setup");
    crate::ops::handle_delete(&mut app).expect("test setup");

    assert!(!temp.path().join("folder").exists());
    let last = recorder.last();
    assert_eq!(last.operation, Operation::Delete);
    assert_eq!(last.total, tally(2, 0), "a delete reads no bytes");
    assert_eq!(last.done, tally(2, 0));
}

#[cfg(unix)]
#[test]
fn a_delete_shown_in_progress_removes_a_link_and_never_its_target() {
    let (recorder, _installed) = Recorder::install();
    let temp = TempDir::new().expect("test setup");
    let kept = temp.path().join("kept");
    tree(&kept);
    let folder = temp.path().join("folder");
    fs::create_dir(&folder).expect("test setup");
    std::os::unix::fs::symlink(&kept, folder.join("link")).expect("test setup");
    let mut app = app_on(temp.path(), "folder", temp.path());

    crate::ops::handle_delete_confirm(&mut app).expect("test setup");
    crate::ops::handle_delete(&mut app).expect("test setup");

    assert!(!folder.exists());
    assert!(kept.join("sub").join("b.txt").exists());
    assert_eq!(recorder.last().total, tally(1, 0), "count the link once");
}

#[test]
fn a_move_counts_the_entries_it_sends() {
    let _serial = RENAME_TEST_MUTEX
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let (recorder, _installed) = Recorder::install();
    let temp = TempDir::new().expect("test setup");
    let (from, to) = (temp.path().join("from"), temp.path().join("to"));
    tree(&from.join("folder"));
    fs::create_dir(&to).expect("test setup");
    let mut app = app_on(&from, "folder", &to);

    crate::ops::handle_move(&mut app).expect("test setup");

    assert!(to.join("folder").join("a.txt").exists());
    let last = recorder.last();
    assert_eq!(last.operation, Operation::Move);
    assert_eq!(
        last.phase,
        Phase::Working,
        "a move measures nothing up front"
    );
    assert_eq!(last.done, tally(1, 0));
    assert_eq!(last.total, tally(1, 0));
}

#[test]
fn a_move_across_filesystems_counts_the_bytes_it_copies() {
    let _serial = RENAME_TEST_MUTEX
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let (recorder, _installed) = Recorder::install();
    let temp = TempDir::new().expect("test setup");
    let (from, to) = (temp.path().join("from"), temp.path().join("to"));
    tree(&from.join("folder"));
    fs::create_dir(&to).expect("test setup");
    let mut app = app_on(&from, "folder", &to);
    let settings = crate::copy::CopyPrompt::for_purpose(crate::copy::Purpose::Move, "folder", &to);

    FORCE_RENAME_FAILURE.store(true, Ordering::SeqCst);
    crate::ops::copy_with(&mut app, &settings);
    FORCE_RENAME_FAILURE.store(false, Ordering::SeqCst);

    assert!(to.join("folder").join("sub").join("b.txt").exists());
    assert!(!from.join("folder").exists());
    let last = recorder.last();
    assert_eq!(last.total, tally(1, 8), "the entry sent, and its bytes");
    assert_eq!(last.done, tally(1, 8));
}
