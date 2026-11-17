//! Purpose: A file sent onto itself, and folder transfers that keep timestamps or nest inside the source.

use super::*;

// ─── a file onto itself ─────────────────────────────────────────────────────

/// Both panels start in the same directory, so the dialog's default
/// destination is often the source's own. "Overwrite" there used to remove the
/// file and then copy from what was no longer there, leaving it empty.
#[test]
fn overwriting_a_file_onto_itself_is_refused_and_the_file_survives() {
    for purpose in [Purpose::Copy, Purpose::Move] {
        let (mut app, from, _to) = app_with(&[("notes.txt", b"payload")]);
        mark(&mut app, "notes.txt");

        let mut dialog = settings(from.path());
        dialog.purpose = purpose;
        dialog.existing = ExistingFiles::Overwrite;
        copy_with(&mut app, &dialog);

        assert_eq!(
            fs::read(from.path().join("notes.txt")).unwrap(),
            b"payload",
            "{purpose:?} onto itself must leave the bytes alone"
        );
        let status = app.status.to_string();
        assert!(
            status.to_lowercase().contains("error"),
            "{purpose:?}: and say why nothing happened, got {status}"
        );
    }
}

/// "Only newer" onto itself is a skip, not a failure: a file is never newer
/// than itself.
#[test]
fn only_newer_onto_itself_is_skipped() {
    let (mut app, from, _to) = app_with(&[("notes.txt", b"payload")]);
    mark(&mut app, "notes.txt");

    let mut dialog = settings(from.path());
    dialog.existing = ExistingFiles::OnlyNewer;
    copy_with(&mut app, &dialog);

    assert_eq!(fs::read(from.path().join("notes.txt")).unwrap(), b"payload");
    assert_eq!(
        app.status,
        Status::Transferred {
            purpose: Purpose::Copy,
            landed: Landed::Nowhere,
            skipped: 1,
            renamed: 0,
        }
    );
}

/// Preserving timestamps applies to a directory as much as to a file, and a
/// directory cannot be opened for writing — so it must not turn a complete
/// copy into a reported failure.
#[test]
fn preserving_timestamps_works_for_a_directory_too() {
    use std::time::{Duration, SystemTime};

    let (mut app, from, to) = app_with(&[]);
    let tree = from.path().join("project");
    fs::create_dir_all(&tree).expect("test setup");
    fs::write(tree.join("a.txt"), b"x").expect("test setup");
    let long_ago = SystemTime::now() - Duration::from_hours(24);
    let times = crate::models::RequestedTimes {
        modified: Some(long_ago),
        accessed: None,
    };
    crate::ops::timestamps::set_times(&tree, times).expect("test setup");
    app.get_active_panel().load_entries().expect("reload");
    mark(&mut app, "project");

    let mut dialog = settings(to.path());
    dialog.timestamps = true;
    copy_with(&mut app, &dialog);

    let copied = to.path().join("project");
    assert_eq!(fs::read(copied.join("a.txt")).unwrap(), b"x");
    let status = app.status.to_string();
    assert!(
        !status.to_lowercase().contains("error"),
        "a complete copy is not a failure, got {status}"
    );
    let source_time = fs::metadata(&tree).unwrap().modified().unwrap();
    let copied_time = fs::metadata(&copied).unwrap().modified().unwrap();
    assert_eq!(copied_time, source_time);
}

/// A move across filesystems with timestamps ticked used to fail after the
/// copy and so never remove the source; with a same-filesystem rename the
/// question does not arise, but the fallback must reach the removal.
#[test]
fn a_move_of_a_directory_with_timestamps_still_removes_the_source() {
    use std::sync::atomic::Ordering;

    let _guard = RENAME_TEST_MUTEX
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let (mut app, from, to) = app_with(&[]);
    let tree = from.path().join("project");
    fs::create_dir_all(&tree).expect("test setup");
    fs::write(tree.join("a.txt"), b"x").expect("test setup");
    app.get_active_panel().load_entries().expect("reload");
    mark(&mut app, "project");

    let mut dialog = settings(to.path());
    dialog.purpose = Purpose::Move;
    dialog.timestamps = true;
    FORCE_RENAME_FAILURE.store(true, Ordering::SeqCst);
    copy_with(&mut app, &dialog);
    FORCE_RENAME_FAILURE.store(false, Ordering::SeqCst);

    assert_eq!(fs::read(to.path().join("project/a.txt")).unwrap(), b"x");
    assert!(!tree.exists(), "the fallback reached the removal");
}

/// A destination that does not exist yet, nested inside the source, must be
/// refused before anything is created — or the refusal leaves a stray
/// directory inside the tree it protected.
#[test]
fn a_nested_destination_inside_the_source_leaves_nothing_behind() {
    let (mut app, from, _to) = app_with(&[]);
    let tree = from.path().join("project");
    fs::create_dir_all(&tree).expect("test setup");
    fs::write(tree.join("a.txt"), b"x").expect("test setup");
    app.get_active_panel().load_entries().expect("reload");
    mark(&mut app, "project");

    let mut dialog = settings(&tree);
    dialog
        .destination
        .set_text(&tree.join("archive/2026").to_string_lossy());
    copy_with(&mut app, &dialog);

    assert!(!tree.join("archive").exists(), "nothing was created inside");
    assert!(
        app.status.to_string().to_lowercase().contains("error"),
        "{}",
        app.status
    );
}
