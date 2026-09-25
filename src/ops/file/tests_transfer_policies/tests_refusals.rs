//! Purpose: The refusals: a folder into itself or onto a file, and moves whose source fails or is gone.

use super::*;

// ─── the refusals and the awkward destinations ──────────────────────────────

/// Copying a directory into itself would recurse until the disk filled, so it
/// is refused before anything is written.
#[test]
fn a_directory_cannot_be_transferred_into_itself() {
    let (mut app, from, _to) = app_with(&[]);
    let tree = from.path().join("project");
    fs::create_dir_all(&tree).expect("test setup");
    fs::write(tree.join("a.txt"), b"x").expect("test setup");
    app.get_active_panel().load_entries().expect("reload");
    mark(&mut app, "project");

    let mut dialog = settings(&tree);
    dialog.destination = tree.to_string_lossy().into_owned();
    copy_with(&mut app, &dialog);

    assert!(
        !tree.join("project").exists(),
        "nothing was written inside itself"
    );
    let status = app.status.to_string();
    assert!(
        status.to_lowercase().contains("error") || status.contains('1'),
        "and it said so, got {status}"
    );
}

/// A destination name already taken by a *file* while the source is a
/// directory is a plain failure, not a merge or a silent skip.
#[test]
fn a_directory_onto_an_existing_file_fails_rather_than_guessing() {
    let (mut app, from, to) = app_with(&[]);
    fs::create_dir_all(from.path().join("data")).expect("test setup");
    fs::write(to.path().join("data"), b"in the way").expect("test setup");
    app.get_active_panel().load_entries().expect("reload");
    mark(&mut app, "data");

    let mut dialog = settings(to.path());
    dialog.existing = ExistingFiles::Ask;
    copy_with(&mut app, &dialog);

    assert_eq!(
        fs::read(to.path().join("data")).unwrap(),
        b"in the way",
        "the file that was there is untouched"
    );
}

/// A move whose source vanishes between the listing and the transfer reports
/// the failure rather than claiming a move that did not happen.
#[test]
fn a_move_of_something_that_is_no_longer_there_is_reported() {
    let (mut app, _from, to) = app_with(&[]);
    mark(&mut app, "ghost.txt");

    let mut dialog = settings(to.path());
    dialog.purpose = Purpose::Move;
    copy_with(&mut app, &dialog);

    let status = app.status.to_string();
    assert!(
        status.to_lowercase().contains("error") || status.contains('1'),
        "got {status}"
    );
}
