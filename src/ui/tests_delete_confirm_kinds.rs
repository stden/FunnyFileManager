//! Unit tests: F8 and Alt+Del ask about what the entry is — a folder, a
//! link or a file — as the panel listed it, the way Far asks.

use std::path::PathBuf;

use ratatui::Terminal;
use tempfile::TempDir;

use crate::models::{App, Overlay, Panel};
use crate::test_support::{buffer_text, WritableTestBackend};
use crate::ui::ui;

/// A panel on a folder holding `folder/`, `file.txt` and `link -> file.txt`.
fn app() -> (App, TempDir) {
    let dir = TempDir::new().expect("temp dir");
    std::fs::create_dir(dir.path().join("folder")).expect("test setup");
    std::fs::write(dir.path().join("file.txt"), b"x").expect("test setup");
    #[cfg(unix)]
    std::os::unix::fs::symlink("file.txt", dir.path().join("link")).expect("test setup");
    let panel = || Panel::new(dir.path().to_path_buf()).expect("panel");
    (App::with_panels(panel(), panel()), dir)
}

fn question(overlay: impl Fn(Vec<PathBuf>) -> Overlay, entry: &str) -> String {
    let (mut app, dir) = app();
    app.set_overlay(overlay(vec![dir.path().join(entry)]));
    let mut terminal = Terminal::new(WritableTestBackend::new(80, 30)).expect("terminal");
    terminal.draw(|f| ui(f, &mut app)).expect("draw");
    buffer_text(&terminal)
}

fn asks(screen: &str, words: &str) {
    assert!(screen.contains(words), "{words}:\n{screen}");
}

#[test]
fn delete_asks_about_a_folder_a_link_or_a_file() {
    asks(
        &question(Overlay::DeleteConfirm, "folder"),
        "Do you wish to delete the folder?",
    );
    asks(
        &question(Overlay::DeleteConfirm, "file.txt"),
        "Do you wish to delete the file?",
    );
    #[cfg(unix)]
    asks(
        &question(Overlay::DeleteConfirm, "link"),
        "Do you wish to delete the link?",
    );
}

#[test]
fn wipe_asks_about_a_folder_a_link_or_a_file() {
    asks(
        &question(Overlay::WipeConfirm, "folder"),
        "Do you wish to wipe the folder?",
    );
    asks(
        &question(Overlay::WipeConfirm, "file.txt"),
        "Do you wish to wipe the file?",
    );
    #[cfg(unix)]
    asks(
        &question(Overlay::WipeConfirm, "link"),
        "Do you wish to wipe the link?",
    );
}

/// A path the panel does not list is asked about as a file, as before.
#[test]
fn an_unlisted_path_is_asked_about_as_a_file() {
    asks(
        &question(Overlay::DeleteConfirm, "gone"),
        "Do you wish to delete the file?",
    );
}

/// The panel answers from its own listing, by the path an operation uses.
#[test]
fn the_panel_knows_the_kind_of_each_listed_path() {
    use crate::models::EntryKind;

    let (app, dir) = app();
    let panel = app.get_active_panel_ref();
    assert_eq!(
        panel.kind_of(&dir.path().join("folder")),
        Some(EntryKind::Directory)
    );
    assert_eq!(
        panel.kind_of(&dir.path().join("file.txt")),
        Some(EntryKind::File)
    );
    assert_eq!(panel.kind_of(&dir.path().join("gone")), None);
}
