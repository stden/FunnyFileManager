//! Purpose: What an F8 reports about itself. Its removal and its panel refresh
//! are two action rows named without any path, and its progress scan counts a
//! link as one entry without walking into its target.

use super::tests_transfer_policies::app_between;
use super::*;

use crate::models::Overlay;
#[cfg(unix)]
use crate::progress::{install_with, Painter, Snapshot, Tally};
#[cfg(unix)]
use std::cell::RefCell;
#[cfg(unix)]
use std::rc::Rc;
#[cfg(unix)]
use std::time::Duration;
use tempfile::TempDir;

/// How many times `action` was recorded, read from its row of the report.
fn recorded(app: &App, action: &str) -> Option<u32> {
    app.performance
        .report()
        .iter()
        .find(|line| line.ends_with(action))?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

/// The total of the last snapshot the progress dialog was asked to draw.
#[cfg(unix)]
#[derive(Clone, Default)]
struct LastTotal(Rc<RefCell<Option<Tally>>>);

#[cfg(unix)]
impl Painter for LastTotal {
    fn paint(&mut self, snapshot: &Snapshot) {
        *self.0.borrow_mut() = Some(snapshot.total);
    }
}

#[test]
fn a_delete_records_its_removal_and_its_refresh_as_their_own_rows() {
    let dir = TempDir::new().expect("test setup");
    let doomed = dir.path().join("doomed-folder");
    fs::create_dir(&doomed).expect("test setup");
    fs::write(doomed.join("inner.txt"), b"x").expect("test setup");
    let mut app = app_between(dir.path(), dir.path());
    app.set_overlay(Overlay::DeleteConfirm(vec![doomed.clone()]));

    handle_delete(&mut app).expect("the delete reports");

    assert!(!doomed.exists());
    assert_eq!(recorded(&app, "Delete/Remove"), Some(1));
    assert_eq!(recorded(&app, "Delete/Refresh"), Some(1));
    assert!(
        !app.performance
            .report()
            .join("\n")
            .contains("doomed-folder"),
        "the rows name what was done, never the path"
    );
}

#[test]
fn a_delete_that_stops_still_records_both_rows() {
    let dir = TempDir::new().expect("test setup");
    let gone = dir.path().join("gone.txt");
    let mut app = app_between(dir.path(), dir.path());
    app.set_overlay(Overlay::DeleteConfirm(vec![gone]));

    handle_delete(&mut app).expect("the batch reports");

    assert_eq!(recorded(&app, "Delete/Remove"), Some(1));
    assert_eq!(recorded(&app, "Delete/Refresh"), Some(1));
}

#[cfg(unix)]
#[test]
fn a_delete_counts_a_link_as_one_entry_and_never_its_target() {
    let last = LastTotal::default();
    let _installed = install_with(Box::new(last.clone()), Duration::ZERO);
    let dir = TempDir::new().expect("test setup");
    let target = dir.path().join("target");
    fs::create_dir(&target).expect("test setup");
    fs::write(target.join("a.txt"), b"a").expect("test setup");
    fs::write(target.join("b.txt"), b"b").expect("test setup");
    let folder = dir.path().join("folder");
    fs::create_dir(&folder).expect("test setup");
    std::os::unix::fs::symlink(&target, folder.join("link")).expect("test setup");
    let mut app = app_between(dir.path(), dir.path());
    app.set_overlay(Overlay::DeleteConfirm(vec![folder.clone()]));

    handle_delete(&mut app).expect("the delete reports");

    assert!(!folder.exists());
    assert!(target.join("b.txt").exists(), "the link's target survives");
    assert_eq!(
        *last.0.borrow(),
        Some(Tally { files: 1, bytes: 0 }),
        "the scan counts the link and no file behind it"
    );
}
