//! Unit tests: a file whose name is not UTF-8 is acted on under its own name.
//!
//! The panel shows such a name lossily (`caf\xE9.txt` as `caf�.txt`). The
//! operations must still reach the file on disk, and never a different file
//! that happens to be shown the same way.

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

use super::tests_transfer_policies::app_between;
use super::*;

use tempfile::TempDir;

fn latin1_name() -> &'static OsStr {
    OsStr::from_bytes(b"caf\xe9.txt")
}

/// F8 on the row at `row` of the left panel, confirmed.
fn delete_row(app: &mut App, row: usize) {
    app.left_panel.select(Some(row));
    handle_delete_confirm(app).expect("confirm");
    handle_delete(app).expect("delete");
}

fn row_of(app: &App, wanted: impl Fn(&crate::models::DirEntry) -> bool) -> usize {
    app.left_panel
        .entries
        .iter()
        .position(wanted)
        .expect("the row is listed")
}

#[test]
fn f8_deletes_a_file_whose_name_is_not_utf8() {
    let dir = TempDir::new().expect("test setup");
    let other = TempDir::new().expect("test setup");
    fs::write(dir.path().join(latin1_name()), b"latin-1").expect("test setup");
    let mut app = app_between(dir.path(), other.path());

    let row = row_of(&app, |entry| entry.name.starts_with("caf"));
    delete_row(&mut app, row);

    assert!(!dir.path().join(latin1_name()).exists(), "{}", app.status);
}

#[test]
fn f8_on_an_undecodable_name_leaves_the_file_shown_the_same_way() {
    let dir = TempDir::new().expect("test setup");
    let other = TempDir::new().expect("test setup");
    fs::write(dir.path().join(latin1_name()), b"latin-1").expect("test setup");
    let lookalike = dir.path().join("caf\u{FFFD}.txt");
    fs::write(&lookalike, b"the other file").expect("test setup");
    let mut app = app_between(dir.path(), other.path());

    let row = row_of(&app, |entry| {
        entry.name.starts_with("caf") && entry.disk_name() == latin1_name()
    });
    delete_row(&mut app, row);

    assert!(!dir.path().join(latin1_name()).exists(), "{}", app.status);
    assert_eq!(fs::read(&lookalike).unwrap(), b"the other file");
}

#[test]
fn a_mark_reaches_every_row_it_shows_on() {
    let dir = TempDir::new().expect("test setup");
    let other = TempDir::new().expect("test setup");
    fs::write(dir.path().join(latin1_name()), b"latin-1").expect("test setup");
    fs::write(dir.path().join("caf\u{FFFD}.txt"), b"lookalike").expect("test setup");
    let mut app = app_between(dir.path(), other.path());
    app.left_panel
        .marked
        .insert(crate::models::FileName::new("caf\u{FFFD}.txt"));

    let mut paths = get_source_paths(&app);
    paths.sort();

    let mut expected = vec![
        dir.path().join(latin1_name()),
        dir.path().join("caf\u{FFFD}.txt"),
    ];
    expected.sort();
    assert_eq!(paths, expected);
}
