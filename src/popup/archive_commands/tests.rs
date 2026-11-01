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

mod extract_and_add;
mod list_and_test;

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
fn a_row_hotkey_runs_it_without_moving_the_cursor() {
    let home = TempDir::new().expect("home");
    let other = TempDir::new().expect("other");
    tar_archive(home.path(), "sample.tgz", "czf");
    let mut app = app_open_on(&home, &other, "sample.tgz");

    press(&mut app, KeyCode::Char('l'));
    assert!(matches!(app.overlay, Overlay::Viewer(_)), "L lists");
}

#[test]
fn a_row_run_on_the_parent_entry_closes_the_menu_and_asks_for_an_archive() {
    let home = TempDir::new().expect("home");
    let other = TempDir::new().expect("other");
    let left = Panel::new(home.path().to_path_buf()).expect("left panel");
    let right = Panel::new(other.path().to_path_buf()).expect("right panel");
    let mut app = App::with_panels(left, right);
    let parent = app
        .get_active_panel_ref()
        .entries
        .iter()
        .position(|entry| entry.name == "..")
        .expect("a temporary folder lists its parent");
    app.get_active_panel().select(Some(parent));
    crate::ops::archive::open_commands(&mut app);

    press(&mut app, KeyCode::Enter);

    assert!(matches!(app.overlay, Overlay::None), "the menu closes");
    assert_eq!(
        app.status,
        Status::Error("Archive commands: put the cursor on an archive".to_owned())
    );
}
