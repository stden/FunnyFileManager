//! Unit tests: the Archive commands menu (Shift+F3) moves, runs each row on
//! the archive under the cursor, and refuses anything that is not one.

use std::path::{Path, PathBuf};
use std::process::Command;

use crossterm::event::KeyCode;
use tempfile::TempDir;

use crate::models::{App, ArchiveCommand, ArchiveMenu, Overlay, Panel, ViewSource};
use crate::ops::archive::{list_archive, test_archive};
use crate::popup::process_popup_input;
use crate::popup::tests::key;
use crate::status::Status;

/// A stored zip holding one file, `hello.txt`, with `hi\n` in it. Written out
/// as bytes so the test needs no `zip` binary to build what `unzip` reads.
const HELLO_ZIP: [u8; 119] = [
    0x50, 0x4b, 0x03, 0x04, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x21, 0x00, 0x7a, 0x7a,
    0x6f, 0xed, 0x03, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x68, 0x65,
    0x6c, 0x6c, 0x6f, 0x2e, 0x74, 0x78, 0x74, 0x68, 0x69, 0x0a, 0x50, 0x4b, 0x01, 0x02, 0x14, 0x03,
    0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x21, 0x00, 0x7a, 0x7a, 0x6f, 0xed, 0x03, 0x00,
    0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x80, 0x01, 0x00, 0x00, 0x00, 0x00, 0x68, 0x65, 0x6c, 0x6c, 0x6f, 0x2e, 0x74, 0x78,
    0x74, 0x50, 0x4b, 0x05, 0x06, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x37, 0x00, 0x00,
    0x00, 0x2a, 0x00, 0x00, 0x00, 0x00, 0x00,
];

fn has_tool(tool: &str) -> bool {
    Command::new(tool)
        .arg("--help")
        .output()
        .is_ok_and(|output| output.status.success())
}

/// A real archive made by `tar`, holding one file, `inside.txt`.
///
/// The member is removed afterwards so the panel lists only the archive and
/// whatever else a test puts beside it.
fn tar_archive(dir: &Path, name: &str, flags: &str) -> PathBuf {
    std::fs::write(dir.join("inside.txt"), "hello\n").expect("member");
    let status = Command::new("tar")
        .current_dir(dir)
        .args([flags, name, "inside.txt"])
        .status()
        .expect("tar runs");
    assert!(status.success(), "tar {flags} {name}");
    std::fs::remove_file(dir.join("inside.txt")).expect("remove member");
    dir.join(name)
}

/// Both panels on real folders, the cursor on `cursor_on`, the menu open.
fn app_open_on(home: &TempDir, other: &TempDir, cursor_on: &str) -> App {
    let left = Panel::new(home.path().to_path_buf()).expect("left panel");
    let right = Panel::new(other.path().to_path_buf()).expect("right panel");
    let mut app = App::with_panels(left, right);
    assert!(
        app.get_active_panel().find_first_matching(cursor_on),
        "{cursor_on} is listed"
    );
    crate::ops::archive::open_commands(&mut app);
    app
}

fn menu(app: &App) -> &ArchiveMenu {
    match &app.overlay {
        Overlay::ArchiveCommands(menu) => menu,
        other => panic!("expected the archive menu, got {}", other.is_active()),
    }
}

fn press(app: &mut App, code: KeyCode) {
    process_popup_input(app, key(code));
}

fn move_to(app: &mut App, command: ArchiveCommand) {
    while menu(app).command() != command {
        press(app, KeyCode::Down);
    }
}

/// Opening sets the variant; Esc takes it down and runs nothing else.
#[test]
fn open_sets_the_variant_and_esc_closes_it() {
    let mut app = App::new().expect("test setup");
    crate::ops::archive::open_commands(&mut app);
    assert!(matches!(app.overlay, Overlay::ArchiveCommands(_)));

    press(&mut app, KeyCode::Char('x'));
    assert!(
        matches!(app.overlay, Overlay::ArchiveCommands(_)),
        "other keys are consumed"
    );

    press(&mut app, KeyCode::Esc);
    assert!(!app.overlay.is_active());
}

#[test]
fn the_cursor_walks_every_row_and_wraps_at_both_ends() {
    let mut app = App::new().expect("test setup");
    crate::ops::archive::open_commands(&mut app);
    assert_eq!(menu(&app).command(), ArchiveCommand::Test);

    let mut seen = vec![menu(&app).command()];
    for _ in 1..ArchiveCommand::ALL.len() {
        press(&mut app, KeyCode::Down);
        seen.push(menu(&app).command());
    }
    assert_eq!(seen, ArchiveCommand::ALL.to_vec(), "Far's order, top down");

    press(&mut app, KeyCode::Down);
    assert_eq!(
        menu(&app).command(),
        ArchiveCommand::Test,
        "wraps to the top"
    );
    press(&mut app, KeyCode::Up);
    assert_eq!(
        menu(&app).command(),
        ArchiveCommand::AddSelected,
        "wraps to the bottom"
    );
    press(&mut app, KeyCode::Home);
    assert_eq!(menu(&app).command(), ArchiveCommand::Test);
    press(&mut app, KeyCode::End);
    assert_eq!(menu(&app).command(), ArchiveCommand::AddSelected);
}

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
fn a_row_hotkey_runs_it_without_moving_the_cursor() {
    let home = TempDir::new().expect("home");
    let other = TempDir::new().expect("other");
    tar_archive(home.path(), "sample.tgz", "czf");
    let mut app = app_open_on(&home, &other, "sample.tgz");

    press(&mut app, KeyCode::Char('l'));
    assert!(matches!(app.overlay, Overlay::Viewer(_)), "L lists");
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
fn extract_here_opens_the_extract_prompt_on_this_folder() {
    let home = TempDir::new().expect("home");
    let other = TempDir::new().expect("other");
    tar_archive(home.path(), "sample.tar", "cf");
    let mut app = app_open_on(&home, &other, "sample.tar");

    move_to(&mut app, ArchiveCommand::ExtractHere);
    press(&mut app, KeyCode::Enter);

    let Overlay::ExtractPrompt(prompt) = &app.overlay else {
        panic!("expected the extract prompt");
    };
    assert_eq!(prompt.destination, home.path().display().to_string());
}

#[test]
fn extract_to_other_panel_opens_the_extract_prompt_on_the_other_folder() {
    let home = TempDir::new().expect("home");
    let other = TempDir::new().expect("other");
    tar_archive(home.path(), "sample.tar", "cf");
    let mut app = app_open_on(&home, &other, "sample.tar");

    move_to(&mut app, ArchiveCommand::ExtractToOtherPanel);
    press(&mut app, KeyCode::Enter);

    let Overlay::ExtractPrompt(prompt) = &app.overlay else {
        panic!("expected the extract prompt");
    };
    assert_eq!(prompt.destination, other.path().display().to_string());
}

/// `extra.txt` marked beside the archive `name`, the cursor back on the
/// archive, and the menu open.
fn app_with_extra_marked(home: &TempDir, other: &TempDir, name: &str) -> App {
    std::fs::write(home.path().join("extra.txt"), "more\n").expect("write");
    let mut app = app_open_on(home, other, "extra.txt");
    app.close_overlay();
    app.get_active_panel().toggle_mark_selected();
    assert!(app.get_active_panel().select_named(name));
    crate::ops::archive::open_commands(&mut app);
    app
}

/// The marked files join the archive under the cursor: the dialog opens on
/// its path with the format fixed, and Enter appends rather than refusing
/// the archive for already existing.
#[test]
fn add_selected_opens_the_add_prompt_named_after_this_archive() {
    let home = TempDir::new().expect("home");
    let other = TempDir::new().expect("other");
    let archive = tar_archive(home.path(), "sample.tar", "cf");
    let mut app = app_with_extra_marked(&home, &other, "sample.tar");

    move_to(&mut app, ArchiveCommand::AddSelected);
    press(&mut app, KeyCode::Enter);

    let Overlay::ArchivePrompt(prompt) = &app.overlay else {
        panic!("expected the add prompt");
    };
    assert_eq!(prompt.name, archive.display().to_string());
    assert!(prompt.append);
    assert_eq!(prompt.sources, ["extra.txt"]);
    assert_eq!(prompt.offered, [crate::models::ArchiveFormat::Tar]);

    press(&mut app, KeyCode::Enter);
    assert!(!app.overlay.is_active());
    assert!(
        matches!(app.status, Status::CommandOutput(_)),
        "{:?}",
        app.status
    );
    let lines = list_archive(&archive).expect("listing");
    assert!(
        lines.iter().any(|line| line.contains("inside.txt")),
        "{lines:?}"
    );
    assert!(
        lines.iter().any(|line| line.contains("extra.txt")),
        "{lines:?}"
    );
}

/// With nothing marked there is nothing to add, and a compressed tar cannot
/// grow: both are said in the status bar instead of opening a dialog that
/// would fail.
#[test]
fn add_selected_needs_marked_files_and_an_archive_that_can_grow() {
    let home = TempDir::new().expect("home");
    let other = TempDir::new().expect("other");
    tar_archive(home.path(), "sample.tar", "cf");
    let mut app = app_open_on(&home, &other, "sample.tar");
    press(&mut app, KeyCode::Char('a'));
    assert!(!app.overlay.is_active());
    assert!(
        matches!(&app.status, Status::Error(text) if text.contains("mark")),
        "{:?}",
        app.status
    );

    tar_archive(home.path(), "packed.tar.gz", "czf");
    let mut app = app_with_extra_marked(&home, &other, "packed.tar.gz");
    press(&mut app, KeyCode::Char('a'));
    assert!(!app.overlay.is_active());
    assert!(
        matches!(&app.status, Status::Error(text) if text.contains("packed.tar.gz")),
        "{:?}",
        app.status
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
