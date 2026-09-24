//! Unit tests: the `NetBox` remote panel draws its session, directory and
//! entries, shows a host's names without obeying them, and neither renderer
//! draws a panel of another kind.

use tempfile::TempDir;

use super::netbox_sessions::{draw, find, terminal};
use crate::models::{App, DirEntry, EntryKind, PanelKind, RemotePanel, RemotePath};
use crate::remote::fake::FakeRemoteFs;
use crate::remote::RemoteLink;
use crate::test_support::buffer_text;
use crate::test_support::netbox::{app_in, name};
use crate::ui::palette;

fn remote_app(dir: &TempDir, listing: Vec<DirEntry>) -> App {
    let mut app = app_in(dir.path());
    let mut panel = RemotePanel::new(
        name("demo@example.test"),
        RemoteLink::new(Box::new(FakeRemoteFs::default())),
    );
    let shown = panel.show(RemotePath::parse("/var/www").expect("path"), listing);
    assert_eq!(
        shown.unusable_names, 0,
        "the fixture listing holds only usable names"
    );
    app.left_panel.kind = PanelKind::Remote(Box::new(panel));
    app
}

fn entry(name: &str, kind: EntryKind) -> DirEntry {
    DirEntry {
        name: name.to_owned(),
        kind,
        ..DirEntry::default()
    }
}

#[test]
fn the_remote_panel_draws_the_session_and_directory_and_its_entries() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = remote_app(
        &dir,
        vec![
            entry("html", EntryKind::Directory),
            entry("a.txt", EntryKind::File),
        ],
    );
    let (screen, terminal) = draw(&mut app, 100, 20);
    for text in [
        " demo@example.test:/var/www ",
        "Name",
        "html",
        "a.txt",
        "/var/www ",
    ] {
        assert!(screen.contains(text), "{text:?} in\n{screen}");
    }
    let (x, y) = find(&screen, "html");
    assert_eq!(terminal.backend().buffer()[(x, y)].fg, palette::DIRECTORY);
    let (x, y) = find(&screen, "a.txt");
    assert_eq!(terminal.backend().buffer()[(x, y)].fg, palette::FILE);
}

/// A host may send any name but `/`; one carrying an escape sequence is
/// shown with a mark where the escape was, not sent to the terminal.
#[test]
fn a_hosts_name_is_shown_and_never_obeyed() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = remote_app(&dir, vec![entry("evil\u{1b}[2Jname", EntryKind::File)]);
    let (screen, _) = draw(&mut app, 100, 20);
    assert!(screen.contains("evil\u{FFFD}[2Jname"), "{screen}");
}

#[test]
fn a_remote_panel_too_short_for_a_row_draws_only_its_frame() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = remote_app(&dir, Vec::new());
    let mut terminal = terminal(60, 3);
    let area = ratatui::layout::Rect::new(0, 0, 60, 3);
    terminal
        .draw(|f| super::remote::render(f, &mut app.left_panel, area, false))
        .expect("draw");
    let screen = buffer_text(&terminal);
    assert!(screen.contains("demo@example.test:/var/www"), "{screen}");
    assert!(!screen.contains("Name"), "{screen}");
}

#[test]
fn a_panel_of_another_kind_is_not_drawn_by_the_netbox_renderers() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    let mut terminal = terminal(60, 10);
    let area = ratatui::layout::Rect::new(0, 0, 60, 10);
    terminal
        .draw(|f| {
            super::sessions::render(f, &mut app.left_panel, area, true);
            super::remote::render(f, &mut app.left_panel, area, true);
        })
        .expect("draw");
    assert!(buffer_text(&terminal).trim().is_empty());
}
