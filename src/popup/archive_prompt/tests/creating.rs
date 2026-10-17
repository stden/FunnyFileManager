//! Purpose: Enter in the Add to archive dialog — the archive made and reported, sources deleted when asked, an existing archive or a failed run kept in the dialog.

use std::path::Path;

use super::*;

/// The last lines of the console transcript.
#[cfg(unix)]
fn transcript(app: &App) -> Vec<String> {
    app.console
        .visible(20)
        .iter()
        .map(|line| line.text().to_owned())
        .collect()
}

/// The file names inside `archive`, as `tar t` lists them.
#[cfg(unix)]
fn tar_listing(archive: &Path) -> String {
    let output = std::process::Command::new("tar")
        .arg("tf")
        .arg(archive)
        .output()
        .expect("tar runs");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// The whole road: Enter packs `notes.txt` into `notes.tar.gz` in the other
/// panel, the transcript shows the command, the status bar says what was
/// added, both panels are re-read, and the source is still there.
#[cfg(unix)]
#[test]
fn enter_creates_the_archive_and_reports_it() {
    let (left, right, mut app) = app_with_notes();
    crate::ops::archive::open_add(&mut app);
    press(&mut app, KeyCode::Enter);

    assert!(!app.overlay.is_active());
    let archive = right.path().join("notes.tar.gz");
    assert!(archive.is_file(), "the archive was created");
    assert!(tar_listing(&archive).contains("notes.txt"));
    assert!(
        left.path().join("notes.txt").exists(),
        "not deleted by default"
    );
    assert!(
        matches!(&app.status, Status::AddedToArchive { archive: a, kept: 0, .. } if *a == archive),
        "{:?}",
        app.status
    );
    let transcript = transcript(&app);
    assert!(
        transcript.iter().any(|line| line.contains("tar ")),
        "the command is echoed: {transcript:?}"
    );
    assert!(
        app.right_panel
            .entries
            .iter()
            .any(|entry| entry.name == "notes.tar.gz"),
        "the other panel shows the archive"
    );
}

/// "Delete files after archiving" removes the originals, but only once the
/// archive exists.
#[cfg(unix)]
#[test]
fn delete_after_archiving_removes_the_sources() {
    let (left, right, mut app) = app_with_notes();
    crate::ops::archive::open_add(&mut app);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Char(' '));
    press(&mut app, KeyCode::Enter);

    assert!(right.path().join("notes.tar.gz").is_file());
    assert!(
        !left.path().join("notes.txt").exists(),
        "the source is gone"
    );
    assert!(
        !app.left_panel
            .entries
            .iter()
            .any(|entry| entry.name == "notes.txt"),
        "the panel was re-read"
    );
}

/// An archive that already exists is never overwritten: the dialog stays up
/// with the name to change and the status bar says why.
#[cfg(unix)]
#[test]
fn an_existing_archive_keeps_the_dialog_open() {
    let (_left, right, mut app) = app_with_notes();
    fs::write(right.path().join("notes.tar.gz"), "old").expect("write");
    crate::ops::archive::open_add(&mut app);
    press(&mut app, KeyCode::Enter);

    assert!(matches!(app.overlay, Overlay::ArchivePrompt(_)));
    assert!(
        matches!(app.status, Status::FileExists(_)),
        "{:?}",
        app.status
    );
    assert_eq!(
        fs::read(right.path().join("notes.tar.gz")).expect("read"),
        b"old"
    );
}

/// A relative name lands in the active panel's folder, the way the copy
/// dialog resolves one.
#[cfg(unix)]
#[test]
fn a_relative_name_lands_in_the_active_panel() {
    let (left, _right, mut app) = app_with_notes();
    crate::ops::archive::open_add(&mut app);
    while !prompt(&app).name.is_empty() {
        press(&mut app, KeyCode::Backspace);
    }
    type_text(&mut app, "here.tar");
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Right);
    assert_eq!(prompt(&app).format, ArchiveFormat::Tar);
    press(&mut app, KeyCode::Enter);
    assert!(left.path().join("here.tar").is_file());
    assert!(tar_listing(&left.path().join("here.tar")).contains("notes.txt"));
}

/// A tool that fails leaves its complaint in the status bar and the
/// transcript rather than a silent nothing.
#[cfg(unix)]
#[test]
fn a_failed_run_reports_the_error() {
    let (_left, _right, mut app) = app_with_notes();
    crate::ops::archive::open_add(&mut app);
    while !prompt(&app).name.is_empty() {
        press(&mut app, KeyCode::Backspace);
    }
    type_text(&mut app, "/nonexistent-dir/out.tar.gz");
    press(&mut app, KeyCode::Enter);
    assert!(!app.overlay.is_active());
    assert!(matches!(app.status, Status::Error(_)), "{:?}", app.status);
}
