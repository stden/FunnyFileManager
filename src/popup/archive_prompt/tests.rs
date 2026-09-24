//! Unit tests: the Add to archive dialog's keys, from opening on Shift+F1 to
//! the archive on disk and its line in the console transcript.

use std::fs;
#[cfg(unix)]
use std::path::Path;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tempfile::TempDir;

use crate::models::{App, ArchiveField, ArchiveFormat, Overlay, Panel, PanelSide};
use crate::popup::process_popup_input;
use crate::popup::tests::key;
use crate::status::Status;

/// Two folders side by side: the left holds `notes.txt`, the right is empty
/// and is where the archive is offered.
fn app_with_notes() -> (TempDir, TempDir, App) {
    let left = TempDir::new().expect("left");
    let right = TempDir::new().expect("right");
    fs::write(left.path().join("notes.txt"), "hello archive\n").expect("write");
    let mut app = App::new().expect("test app");
    app.left_panel = Panel::new(left.path().to_path_buf()).expect("left panel");
    app.right_panel = Panel::new(right.path().to_path_buf()).expect("right panel");
    app.active_panel = PanelSide::Left;
    assert!(app.left_panel.select_named("notes.txt"));
    (left, right, app)
}

fn prompt(app: &App) -> &crate::models::ArchivePrompt {
    match &app.overlay {
        Overlay::ArchivePrompt(prompt) => prompt,
        _ => panic!("expected the archive prompt"),
    }
}

fn press(app: &mut App, code: KeyCode) {
    process_popup_input(app, key(code));
}

fn type_text(app: &mut App, text: &str) {
    for ch in text.chars() {
        press(app, KeyCode::Char(ch));
    }
}

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

/// Opening offers `notes.tar.gz` in the other panel; Esc takes the dialog
/// down and creates nothing.
#[test]
fn open_offers_the_stem_in_the_other_panel_and_esc_closes_it() {
    let (_left, right, mut app) = app_with_notes();
    crate::ops::archive::open_add(&mut app);
    let expected = right.path().join("notes.tar.gz");
    assert_eq!(prompt(&app).name, expected.display().to_string());
    assert_eq!(prompt(&app).sources, ["notes.txt"]);

    press(&mut app, KeyCode::Char('x'));
    assert!(
        matches!(app.overlay, Overlay::ArchivePrompt(_)),
        "other keys are consumed"
    );
    press(&mut app, KeyCode::Esc);
    assert!(!app.overlay.is_active());
    assert!(!expected.exists());
}

/// Nothing under the cursor but `..`: the row still says something.
#[test]
fn open_with_nothing_to_archive_reports_and_stays_closed() {
    let left = TempDir::new().expect("left");
    let mut app = App::new().expect("test app");
    app.left_panel = Panel::new(left.path().to_path_buf()).expect("left panel");
    app.active_panel = PanelSide::Left;
    crate::ops::archive::open_add(&mut app);
    assert!(!app.overlay.is_active());
    assert!(matches!(app.status, Status::Error(_)), "{:?}", app.status);
}

/// Typing edits the name, Backspace takes it back, and the modifier keys are
/// not letters.
#[test]
fn typing_edits_the_name_field() {
    let (_left, _right, mut app) = app_with_notes();
    crate::ops::archive::open_add(&mut app);
    press(&mut app, KeyCode::Backspace);
    press(&mut app, KeyCode::Backspace);
    type_text(&mut app, "xz");
    assert!(prompt(&app).name.ends_with("notes.tar.xz"));

    process_popup_input(
        &mut app,
        KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL),
    );
    assert!(
        prompt(&app).name.ends_with("notes.tar.xz"),
        "Ctrl+Q is not a q"
    );
}

/// Tab reaches the radio, Right turns it and renames the archive, Tab reaches
/// the checkbox and Space ticks it.
#[test]
fn tab_right_and_space_drive_the_controls() {
    let (_left, _right, mut app) = app_with_notes();
    crate::ops::archive::open_add(&mut app);
    press(&mut app, KeyCode::Tab);
    assert_eq!(prompt(&app).field, ArchiveField::Format);
    press(&mut app, KeyCode::Right);
    assert_eq!(prompt(&app).format, ArchiveFormat::Tar);
    assert!(prompt(&app).name.ends_with("notes.tar"));
    press(&mut app, KeyCode::Left);
    assert_eq!(prompt(&app).format, ArchiveFormat::TarGz);

    press(&mut app, KeyCode::Down);
    assert_eq!(prompt(&app).field, ArchiveField::DeleteAfter);
    press(&mut app, KeyCode::Char(' '));
    assert!(prompt(&app).delete_after);
    press(&mut app, KeyCode::Up);
    assert_eq!(prompt(&app).field, ArchiveField::Format);
    press(&mut app, KeyCode::BackTab);
    assert_eq!(prompt(&app).field, ArchiveField::Name);
}

/// Enter on Cancel is a cancel wherever else it would be a go.
#[test]
fn enter_on_cancel_creates_nothing() {
    let (_left, right, mut app) = app_with_notes();
    crate::ops::archive::open_add(&mut app);
    for _ in 0..4 {
        press(&mut app, KeyCode::Tab);
    }
    assert_eq!(prompt(&app).field, ArchiveField::Cancel);
    press(&mut app, KeyCode::Enter);
    assert!(!app.overlay.is_active());
    assert!(!right.path().join("notes.tar.gz").exists());
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
