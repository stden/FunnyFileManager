//! Purpose: A transfer onto itself is refused, and the status line says what really happened.

use super::*;

// ─── onto itself ────────────────────────────────────────────────────────────

/// Both panels on one folder is the dialog's default destination. Far refuses
/// a copy or a move onto itself ("You cannot copy or move a file or folder
/// onto itself") whatever the "already existing files" choice; with Ask, F6
/// used to rename the file to `name_1` and report it as copied.
#[test]
fn a_transfer_onto_itself_is_refused_and_the_file_keeps_its_name() {
    for purpose in [Purpose::Copy, Purpose::Move] {
        let (mut app, from, _to) = app_with(&[("utf16le.txt", b"payload")]);
        mark(&mut app, "utf16le.txt");

        let mut dialog = settings(from.path());
        dialog.purpose = purpose;
        dialog.existing = ExistingFiles::Ask;
        copy_with(&mut app, &dialog);

        assert_eq!(
            fs::read(from.path().join("utf16le.txt")).expect("still there"),
            b"payload",
            "{purpose:?}"
        );
        assert!(
            !from.path().join("utf16le_1.txt").exists(),
            "{purpose:?} made or renamed to a second name"
        );
        let status = app.status.to_string();
        assert!(status.contains("onto itself"), "{purpose:?}: {status}");
    }
}

/// A folder onto itself is the same refusal, not a duplicate `name_1` tree.
#[test]
fn a_folder_onto_itself_is_refused_too() {
    let (mut app, from, _to) = app_with(&[]);
    fs::create_dir(from.path().join("project")).expect("test setup");
    mark(&mut app, "project");

    let mut dialog = settings(from.path());
    dialog.purpose = Purpose::Move;
    copy_with(&mut app, &dialog);

    assert!(from.path().join("project").is_dir());
    assert!(!from.path().join("project_1").exists());
    assert!(app.status.to_string().contains("onto itself"));
}

// ─── the status line ────────────────────────────────────────────────────────

/// F6 says it moved, and a failed F6 says "Moved … error(s)", not "Copied".
#[test]
fn a_move_is_reported_as_a_move() {
    let (mut app, _from, to) = app_with(&[("notes.txt", b"x")]);
    mark(&mut app, "notes.txt");
    mark(&mut app, "vanished.txt");

    let mut dialog = settings(to.path());
    dialog.purpose = Purpose::Move;
    copy_with(&mut app, &dialog);

    let status = app.status.to_string();
    assert!(
        status.starts_with("Moved 1 item(s), 1 error(s)"),
        "{status}"
    );
}

/// "Existing files kept, … renamed" only when something was in the way: a
/// plain transfer into an empty folder renamed nothing.
#[test]
fn renaming_is_mentioned_only_when_it_happened() {
    for purpose in [Purpose::Copy, Purpose::Move] {
        let (mut app, _from, to) = app_with(&[("notes.txt", b"x")]);
        mark(&mut app, "notes.txt");

        let mut dialog = settings(to.path());
        dialog.purpose = purpose;
        dialog.existing = ExistingFiles::Ask;
        copy_with(&mut app, &dialog);

        let status = app.status.to_string();
        let verb = match purpose {
            Purpose::Copy => "Copied to ",
            Purpose::Move => "Moved to ",
        };
        assert!(status.starts_with(verb), "{status}");
        assert!(!status.contains("renamed"), "nothing was renamed: {status}");
    }
}
