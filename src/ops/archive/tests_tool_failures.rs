//! Purpose: An archive tool that cannot start, or that refuses what it is given,
//! is reported with what happened, and leaves no finished-looking archive behind.

use std::fs;

use super::*;
use crate::models::{ArchiveFormat, Panel};
use tempfile::TempDir;

fn app_on(root: &Path) -> App {
    App::with_panels(
        Panel::new(root.to_path_buf()).unwrap(),
        Panel::new(root.to_path_buf()).unwrap(),
    )
}

#[test]
fn an_archive_whose_working_folder_has_gone_is_reported_and_not_left_behind() {
    let root = TempDir::new().unwrap();
    fs::write(root.path().join("a.txt"), b"x").unwrap();
    let mut app = app_on(root.path());
    let gone = root.path().join("vanished");
    app.left_panel.path = gone.clone();
    let prompt = ArchivePrompt::new(
        vec!["a.txt".to_owned()],
        "out",
        root.path(),
        vec![ArchiveFormat::Tar],
    );

    add(&mut app, &prompt);

    assert!(matches!(app.status, Status::Error(_)), "{}", app.status);
    assert!(!gone.exists());
    assert!(!root.path().join("out.tar").exists());
}

#[test]
fn a_corrupt_archive_is_refused_and_nothing_is_unpacked() {
    let root = TempDir::new().unwrap();
    let archive = root.path().join("broken.tar");
    fs::write(&archive, b"this is not a tar archive at all").unwrap();
    let destination = root.path().join("out");
    fs::create_dir(&destination).unwrap();

    assert!(extract_archive(&archive, &destination).is_err());
    assert_eq!(fs::read_dir(&destination).unwrap().count(), 0);
}

#[test]
fn an_unknown_archive_format_is_named_in_the_transcript() {
    let root = TempDir::new().unwrap();
    let archive = root.path().join("data.bin");
    fs::write(&archive, b"x").unwrap();
    let mut app = app_on(root.path());
    app.overlay = Overlay::ExtractPrompt(Box::new(ExtractPrompt::new(
        vec![archive.clone()],
        root.path(),
    )));

    run_extract(&mut app);

    let shown = format!("extract {}", archive.display());
    assert!(
        app.console
            .visible(20)
            .iter()
            .any(|line| line.text().contains(&shown)),
        "the transcript names the unknown archive"
    );
}
