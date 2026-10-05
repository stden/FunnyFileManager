//! Purpose: an archive run that fails leaves no half-written archive of its
//! own behind, and never touches one that was there before.

use super::*;

/// Mark a source that is not there beside `notes.txt`: tar packs what it
/// can, then fails, as it does on a file it may not read.
fn with_a_missing_source(app: &mut App) {
    let Overlay::ArchivePrompt(prompt) = &mut app.overlay else {
        panic!("the archive prompt is open");
    };
    prompt.sources.push("gone.txt".to_owned());
}

/// The guest sweep's case: a failed tar run left a 131-byte archive holding
/// only the files it could read.
#[test]
fn a_failed_run_removes_the_archive_it_began() {
    let (left, right, mut app) = app_with_notes();
    crate::ops::archive::open_add(&mut app);
    with_a_missing_source(&mut app);

    press(&mut app, KeyCode::Enter);

    assert!(matches!(app.status, Status::Error(_)), "{:?}", app.status);
    let archive = right.path().join("notes.tar.gz");
    assert!(!archive.exists(), "the partial archive is gone");
    assert!(left.path().join("notes.txt").exists(), "the source stays");
}

/// Adding to an archive that was already there: a failure may have changed
/// it, but removing it would lose what it held before.
#[test]
fn a_failed_append_keeps_the_archive_that_was_there() {
    let (left, _right, mut app) = app_with_notes();
    let archive = left.path().join("kept.tar");
    let first = crate::ops::archive::ArchiveRequest {
        archive: &archive,
        format: ArchiveFormat::Tar,
        sources: &["notes.txt".to_owned()],
        append: false,
    };
    crate::ops::archive::create_archive(&first, left.path()).expect("tar runs");
    let sources = vec!["notes.txt".to_owned()];
    let prompt = crate::models::ArchivePrompt::appending(sources, &archive, ArchiveFormat::Tar);
    app.set_overlay(Overlay::ArchivePrompt(Box::new(prompt)));
    with_a_missing_source(&mut app);

    press(&mut app, KeyCode::Enter);

    assert!(matches!(app.status, Status::Error(_)), "{:?}", app.status);
    assert!(archive.is_file(), "the archive that was there stays");
}
