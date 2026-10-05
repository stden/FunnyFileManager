//! Purpose: the menu's List contents and Test archive rows — a tar and a zip read, a damaged archive named, and a plain file refused.

use super::*;

#[test]
fn list_contents_opens_the_viewer_on_the_listing() {
    let home = TempDir::new().expect("home");
    let other = TempDir::new().expect("other");
    let archive = tar_archive(home.path(), "sample.tar", "cf");
    let mut app = app_open_on(&home, &other, "sample.tar");

    move_to(&mut app, ArchiveCommand::List);
    press(&mut app, KeyCode::Enter);

    let Overlay::Viewer(viewer) = &app.overlay else {
        panic!("expected the viewer");
    };
    assert_eq!(
        viewer.source,
        ViewSource::Report(archive.display().to_string()),
        "titled after the archive"
    );
    assert!(
        viewer.lines.iter().any(|line| line.contains("inside.txt")),
        "{:?}",
        viewer.lines
    );
    assert_eq!(app.status, Status::OpenedView("sample.tar".to_owned()));
}

#[test]
fn test_archive_reports_a_sound_archive_in_the_status_bar() {
    let home = TempDir::new().expect("home");
    let other = TempDir::new().expect("other");
    tar_archive(home.path(), "sound.tar.gz", "czf");
    let mut app = app_open_on(&home, &other, "sound.tar.gz");

    press(&mut app, KeyCode::Enter);

    assert!(!app.overlay.is_active(), "the menu is gone");
    assert_eq!(
        app.status,
        Status::CommandOutput("Archive OK: sound.tar.gz".to_owned())
    );
}

#[test]
fn test_archive_names_a_damaged_archive() {
    let home = TempDir::new().expect("home");
    let other = TempDir::new().expect("other");
    std::fs::write(home.path().join("broken.tar.gz"), "this is not gzip\n").expect("write");
    let mut app = app_open_on(&home, &other, "broken.tar.gz");

    press(&mut app, KeyCode::Enter);

    assert!(!app.overlay.is_active());
    let Status::Error(text) = &app.status else {
        panic!("expected an error, got {:?}", app.status);
    };
    assert!(text.starts_with("broken.tar.gz: "), "{text}");
    assert!(
        app.console
            .visible(20)
            .iter()
            .any(|line| line.text().contains("broken.tar.gz")),
        "the tool's own words reach the console transcript"
    );
}

#[test]
fn a_file_that_is_not_an_archive_is_refused() {
    let home = TempDir::new().expect("home");
    let other = TempDir::new().expect("other");
    std::fs::write(home.path().join("notes.txt"), "plain\n").expect("write");
    let mut app = app_open_on(&home, &other, "notes.txt");

    move_to(&mut app, ArchiveCommand::List);
    press(&mut app, KeyCode::Enter);

    assert!(!app.overlay.is_active(), "no viewer, no menu");
    let Status::Error(text) = &app.status else {
        panic!("expected an error, got {:?}", app.status);
    };
    assert!(text.contains("notes.txt"), "{text}");
    assert!(
        text.contains(".zip"),
        "names what would be accepted: {text}"
    );
}

#[test]
fn list_archive_reads_a_tar_and_refuses_a_plain_file() {
    let dir = TempDir::new().expect("dir");
    let archive = tar_archive(dir.path(), "sample.tar.xz", "cJf");
    let lines = list_archive(&archive).expect("listing");
    assert!(
        lines.iter().any(|line| line.contains("inside.txt")),
        "{lines:?}"
    );

    let plain = dir.path().join("readme.md");
    std::fs::write(&plain, "# hi\n").expect("write");
    let error = list_archive(&plain).expect_err("not an archive");
    assert!(
        error.to_string().contains(".zip"),
        "names what is accepted: {error}"
    );
    assert!(test_archive(&plain).is_err());
}

#[test]
fn list_and_test_read_a_zip_through_unzip() {
    if !has_tool("unzip") {
        eprintln!("unzip is not installed here; skipping");
        return;
    }
    let dir = TempDir::new().expect("dir");
    let archive = dir.path().join("hello.zip");
    std::fs::write(&archive, HELLO_ZIP).expect("write zip");

    let lines = list_archive(&archive).expect("listing");
    assert!(
        lines.iter().any(|line| line.contains("hello.txt")),
        "{lines:?}"
    );
    test_archive(&archive).expect("a sound zip");

    std::fs::write(&archive, b"PK\x03\x04 truncated").expect("damage it");
    assert!(
        test_archive(&archive).is_err(),
        "a damaged zip fails the test"
    );
}
