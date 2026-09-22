//! Unit tests: Alt+Del wipes the way Far describes — zeros over every byte,
//! then a truncation, a temporary name, and only then removal; directories
//! recurse and links are unlinked, never followed.
//!
//! The zeros are the point and the hardest thing to see: the next step erases
//! them. The `WITNESS` seam is the one moment they can be read back.

use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use tempfile::TempDir;

use super::{cancel, open, run, wipe_path, Stage, WITNESS};
use crate::models::{App, FileName, Overlay, Panel};
use crate::status::Status;

/// What the seam saw: each stage with the path it was called for, and the
/// bytes on disk at that moment.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Seen {
    stage: Stage,
    path: PathBuf,
    bytes: Vec<u8>,
}

/// Installs the witness for one test and takes it down with the test, so a
/// failing assertion cannot leave it behind for the next test on this thread.
struct Witness(Rc<RefCell<Vec<Seen>>>);

impl Witness {
    fn install() -> Self {
        let log: Rc<RefCell<Vec<Seen>>> = Rc::default();
        let sink = Rc::clone(&log);
        WITNESS.with(|witness| {
            *witness.borrow_mut() = Some(Box::new(move |stage: Stage, path: &Path| {
                let bytes = fs::read(path).unwrap_or_default();
                sink.borrow_mut().push(Seen {
                    stage,
                    path: path.to_path_buf(),
                    bytes,
                });
            }));
        });
        Self(log)
    }

    fn seen(&self) -> Vec<Seen> {
        self.0.borrow().clone()
    }

    fn at(&self, stage: Stage) -> Vec<Seen> {
        self.seen()
            .into_iter()
            .filter(|seen| seen.stage == stage)
            .collect()
    }
}

impl Drop for Witness {
    fn drop(&mut self) {
        WITNESS.with(|witness| *witness.borrow_mut() = None);
    }
}

fn app_in(dir: &Path) -> App {
    let left = Panel::new(dir.to_path_buf()).expect("left panel");
    let right = Panel::new(dir.to_path_buf()).expect("right panel");
    App::with_panels(left, right)
}

fn status_text(app: &App) -> String {
    app.status.to_string()
}

#[test]
fn a_file_is_zeroed_before_it_is_removed() {
    let dir = TempDir::new().expect("temp dir");
    let file = dir.path().join("secret.txt");
    fs::write(&file, b"top secret payload").expect("write");
    let witness = Witness::install();

    wipe_path(&file).expect("wipe");

    let overwritten = witness.at(Stage::Overwritten);
    assert_eq!(overwritten.len(), 1, "{:?}", witness.seen());
    assert_eq!(overwritten[0].path, file);
    assert_eq!(overwritten[0].bytes.len(), b"top secret payload".len());
    assert!(overwritten[0].bytes.iter().all(|&byte| byte == 0));
    assert!(!file.exists(), "the file is gone afterwards");
}

#[test]
fn the_zero_pass_covers_files_longer_than_one_buffer() {
    let dir = TempDir::new().expect("temp dir");
    let file = dir.path().join("large.bin");
    let payload: Vec<u8> = (0..(super::ZERO_CHUNK * 3 + 7))
        .map(|i| (i % 251) as u8 + 1)
        .collect();
    fs::write(&file, &payload).expect("write");
    let witness = Witness::install();

    wipe_path(&file).expect("wipe");

    let overwritten = witness.at(Stage::Overwritten);
    assert_eq!(overwritten[0].bytes.len(), payload.len());
    assert!(overwritten[0].bytes.iter().all(|&byte| byte == 0));
}

#[test]
fn an_empty_file_is_wiped_without_a_zero_to_write() {
    let dir = TempDir::new().expect("temp dir");
    let file = dir.path().join("empty");
    fs::write(&file, b"").expect("write");
    let witness = Witness::install();

    wipe_path(&file).expect("wipe");

    assert_eq!(witness.at(Stage::Overwritten)[0].bytes, Vec::<u8>::new());
    assert!(!file.exists());
}

#[test]
fn the_last_name_is_a_temporary_one_and_the_file_is_empty_by_then() {
    let dir = TempDir::new().expect("temp dir");
    let file = dir.path().join("named.txt");
    fs::write(&file, b"something").expect("write");
    let witness = Witness::install();

    wipe_path(&file).expect("wipe");

    let renamed = witness.at(Stage::Renamed);
    assert_eq!(renamed.len(), 1, "{:?}", witness.seen());
    assert_ne!(
        renamed[0].path, file,
        "the real name is gone before removal"
    );
    assert_eq!(renamed[0].path.parent(), Some(dir.path()));
    assert!(
        renamed[0]
            .path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with(super::TEMP_PREFIX)),
        "{:?}",
        renamed[0].path
    );
    assert!(renamed[0].bytes.is_empty(), "truncated before the rename");
}

#[test]
fn a_directory_is_wiped_file_by_file_and_then_removed() {
    let dir = TempDir::new().expect("temp dir");
    let root = dir.path().join("project");
    let nested = root.join("src").join("deep");
    fs::create_dir_all(&nested).expect("mkdir");
    fs::write(root.join("a.txt"), b"aaaa").expect("write");
    fs::write(nested.join("b.txt"), b"bbbbbb").expect("write");
    let witness = Witness::install();

    wipe_path(&root).expect("wipe");

    let overwritten = witness.at(Stage::Overwritten);
    assert_eq!(overwritten.len(), 2, "{:?}", witness.seen());
    assert!(overwritten
        .iter()
        .all(|seen| seen.bytes.iter().all(|&byte| byte == 0)));
    // Two files and three directories each carried a temporary name last.
    assert_eq!(witness.at(Stage::Renamed).len(), 5, "{:?}", witness.seen());
    assert!(!root.exists());
    assert!(
        fs::read_dir(dir.path()).expect("read dir").next().is_none(),
        "no temporary names left behind"
    );
}

#[cfg(unix)]
#[test]
fn a_symlink_is_unlinked_and_its_target_is_untouched() {
    let dir = TempDir::new().expect("temp dir");
    let target = dir.path().join("target.txt");
    fs::write(&target, b"keep me").expect("write");
    let link = dir.path().join("link");
    std::os::unix::fs::symlink(&target, &link).expect("symlink");
    let witness = Witness::install();

    wipe_path(&link).expect("wipe");

    assert!(witness.at(Stage::Overwritten).is_empty(), "never followed");
    assert!(fs::symlink_metadata(&link).is_err(), "the link is gone");
    assert_eq!(fs::read(&target).expect("read"), b"keep me");
}

#[test]
fn a_missing_entry_is_an_error() {
    let dir = TempDir::new().expect("temp dir");
    let error = wipe_path(&dir.path().join("absent")).expect_err("must fail");
    assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
}

#[test]
fn open_asks_about_the_selected_entry() {
    let dir = TempDir::new().expect("temp dir");
    fs::write(dir.path().join("one.txt"), b"1").expect("write");
    let mut app = app_in(dir.path());
    assert!(app.get_active_panel().select_named("one.txt"));

    open(&mut app);

    let Overlay::WipeConfirm(paths) = &app.overlay else {
        panic!("no wipe dialog");
    };
    assert_eq!(paths, &[dir.path().join("one.txt")]);
}

#[test]
fn open_asks_about_every_marked_entry() {
    let dir = TempDir::new().expect("temp dir");
    fs::write(dir.path().join("one.txt"), b"1").expect("write");
    fs::write(dir.path().join("two.txt"), b"2").expect("write");
    let mut app = app_in(dir.path());
    app.get_active_panel()
        .marked
        .insert(FileName::from("one.txt"));
    app.get_active_panel()
        .marked
        .insert(FileName::from("two.txt"));

    open(&mut app);

    let Overlay::WipeConfirm(paths) = &app.overlay else {
        panic!("no wipe dialog");
    };
    assert_eq!(paths.len(), 2);
}

#[test]
fn open_with_the_cursor_on_dotdot_says_there_is_nothing_to_wipe() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    assert!(app.get_active_panel().select_named(".."));

    open(&mut app);

    assert!(!app.overlay.is_active(), "no dialog about nothing");
    assert_eq!(status_text(&app), "Nothing to wipe");
}

#[test]
fn run_wipes_every_confirmed_entry_clears_marks_and_reports() {
    let dir = TempDir::new().expect("temp dir");
    fs::write(dir.path().join("one.txt"), b"1").expect("write");
    fs::write(dir.path().join("two.txt"), b"2").expect("write");
    fs::write(dir.path().join("keep.txt"), b"k").expect("write");
    let mut app = app_in(dir.path());
    app.get_active_panel()
        .marked
        .insert(FileName::from("one.txt"));
    app.get_active_panel()
        .marked
        .insert(FileName::from("two.txt"));
    open(&mut app);

    run(&mut app);

    assert!(!app.overlay.is_active());
    assert!(!dir.path().join("one.txt").exists());
    assert!(!dir.path().join("two.txt").exists());
    assert!(dir.path().join("keep.txt").exists());
    assert!(app.get_active_panel().marked.is_empty());
    assert_eq!(status_text(&app), "Wiped 2 item(s)");
    let listed: Vec<&str> = app
        .get_active_panel_ref()
        .entries
        .iter()
        .map(|entry| entry.name.as_str())
        .collect();
    assert!(!listed.contains(&"one.txt"), "panel reloaded: {listed:?}");
}

#[test]
fn run_folds_a_failure_into_the_summary_and_wipes_the_rest() {
    let dir = TempDir::new().expect("temp dir");
    fs::write(dir.path().join("one.txt"), b"1").expect("write");
    let mut app = app_in(dir.path());
    app.set_overlay(Overlay::WipeConfirm(vec![
        dir.path().join("absent"),
        dir.path().join("one.txt"),
    ]));

    run(&mut app);

    assert!(!dir.path().join("one.txt").exists());
    let status = status_text(&app);
    assert!(
        status.starts_with("Wiped 1 item(s), 1 error(s): "),
        "{status}"
    );
    assert!(status.contains("absent"), "{status}");
}

#[test]
fn run_does_nothing_unless_the_wipe_dialog_is_up() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    app.set_overlay(Overlay::DeleteConfirm(vec![dir.path().join("x")]));

    run(&mut app);

    assert!(matches!(app.overlay, Overlay::DeleteConfirm(_)));
    assert!(matches!(app.status, Status::KeyHints));
}

#[test]
fn cancel_closes_the_dialog_and_touches_nothing() {
    let dir = TempDir::new().expect("temp dir");
    fs::write(dir.path().join("one.txt"), b"1").expect("write");
    let mut app = app_in(dir.path());
    app.set_overlay(Overlay::WipeConfirm(vec![dir.path().join("one.txt")]));

    cancel(&mut app);

    assert!(!app.overlay.is_active());
    assert_eq!(fs::read(dir.path().join("one.txt")).expect("read"), b"1");
    assert_eq!(status_text(&app), "Wipe cancelled");
}
