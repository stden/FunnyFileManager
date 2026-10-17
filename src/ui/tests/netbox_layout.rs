//! Unit tests: where exactly the `NetBox` sessions and remote panels put
//! things — the heading row, each list row, the info line, the frame — and
//! their colours. The dialogs are in `netbox_layout_dialogs`.

use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::Terminal;
use tempfile::TempDir;

use crate::models::{App, DirEntry, EntryKind, PanelKind, RemotePanel, RemotePath, RowMotion};
use crate::remote::fake::FakeRemoteFs;
use crate::remote::RemoteLink;
use crate::sessions::SessionStore;
use crate::test_support::netbox::{app_in, app_on_sessions, name, session};
use crate::test_support::{buffer_text, WritableTestBackend};
use crate::ui::{palette, FAR_BLUE};

/// A 40 by 10 panel: the frame, a heading, six rows, the info line.
pub(super) const PANEL: Rect = Rect::new(0, 0, 40, 10);

pub(super) type Screen = (Vec<String>, Terminal<WritableTestBackend>);

pub(super) fn lines(terminal: &Terminal<WritableTestBackend>) -> Vec<String> {
    buffer_text(terminal).lines().map(str::to_owned).collect()
}

fn draw_sessions(app: &mut App, is_active: bool) -> Screen {
    let mut terminal = Terminal::new(WritableTestBackend::new(40, 10)).expect("terminal");
    terminal
        .draw(|f| super::sessions::render(f, &mut app.left_panel, PANEL, is_active, None))
        .expect("draw");
    (lines(&terminal), terminal)
}

fn draw_remote(app: &mut App, is_active: bool) -> Screen {
    let mut terminal = Terminal::new(WritableTestBackend::new(40, 10)).expect("terminal");
    terminal
        .draw(|f| super::remote::render(f, &mut app.left_panel, PANEL, is_active, None))
        .expect("draw");
    (lines(&terminal), terminal)
}

pub(super) fn sessions_app(dir: &TempDir, hosts: &[&str]) -> App {
    let mut store = SessionStore::default();
    for host in hosts {
        store.upsert(session(host, None));
    }
    app_on_sessions(dir.path(), store)
}

pub(super) fn move_sessions(app: &mut App, motion: RowMotion) {
    if let PanelKind::Sessions(panel) = &mut app.left_panel.kind {
        panel.move_cursor(motion);
    }
}

#[test]
fn the_sessions_panel_puts_each_part_on_its_row() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = sessions_app(&dir, &["a.test", "b.test"]);
    let (rows, _) = draw_sessions(&mut app, true);
    assert!(
        rows[0].contains(" NetBox: Stored sessions [/] "),
        "{rows:?}"
    );
    assert!(rows[1].starts_with("║Session name"), "{rows:?}");
    assert!(rows[2].starts_with("║.. "), "{rows:?}");
    assert!(rows[3].starts_with("║a.test "), "{rows:?}");
    assert!(rows[4].starts_with("║b.test "), "{rows:?}");
    assert!(rows[5].trim_matches(['║', ' ']).is_empty(), "{rows:?}");
    assert!(rows[8].starts_with("║2 sessions "), "{rows:?}");
    assert!(
        rows[9].starts_with('╚') && rows[9].ends_with('╝'),
        "{rows:?}"
    );
}

#[test]
fn a_long_list_shows_its_last_six_rows_under_the_heading() {
    let dir = TempDir::new().expect("temp dir");
    let hosts: Vec<String> = (0..12).map(|index| format!("h{index:02}.test")).collect();
    let hosts: Vec<&str> = hosts.iter().map(String::as_str).collect();
    let mut app = sessions_app(&dir, &hosts);
    move_sessions(&mut app, RowMotion::Last);
    let (rows, _) = draw_sessions(&mut app, true);
    assert!(rows[1].starts_with("║Session name"), "{rows:?}");
    assert!(rows[2].starts_with("║h06.test"), "{rows:?}");
    assert!(rows[7].starts_with("║h11.test"), "{rows:?}");
    assert!(rows[8].starts_with("║12 sessions"), "{rows:?}");
    let PanelKind::Sessions(panel) = &app.left_panel.kind else {
        panic!("the sessions list");
    };
    assert_eq!(panel.scroll, 7);
}

#[test]
fn the_rows_are_coloured_by_kind_and_the_cursor_marks_the_active_panel_only() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = sessions_app(&dir, &["a.test", "b.test"]);
    move_sessions(&mut app, RowMotion::Next);
    let (_, terminal) = draw_sessions(&mut app, true);
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(1, 2)].fg, palette::DIRECTORY);
    assert!(buffer[(1, 2)].modifier.contains(Modifier::BOLD));
    assert_eq!(buffer[(1, 3)].bg, palette::CURSOR_BG);
    assert_eq!(buffer[(1, 3)].fg, palette::CURSOR_FG);
    assert_eq!(buffer[(1, 4)].fg, palette::FILE);
    assert_ne!(buffer[(1, 4)].bg, palette::CURSOR_BG);

    let (_, terminal) = draw_sessions(&mut app, false);
    assert_ne!(terminal.backend().buffer()[(1, 3)].bg, palette::CURSOR_BG);
}

#[test]
fn the_active_panels_frame_is_bold_and_its_title_highlighted() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = sessions_app(&dir, &["a.test"]);
    let (rows, terminal) = draw_sessions(&mut app, true);
    let (before_title, _) = rows[0].split_once("NetBox").expect("the title");
    let title = u16::try_from(before_title.chars().count()).expect("fits");
    let buffer = terminal.backend().buffer();
    assert!(buffer[(0, 5)].modifier.contains(Modifier::BOLD));
    assert_eq!(buffer[(0, 5)].fg, palette::FRAME);
    assert_eq!(buffer[(title, 0)].bg, palette::HIGHLIGHT_BG);
    assert_eq!(buffer[(title, 0)].fg, palette::HIGHLIGHT_FG);

    let (_, terminal) = draw_sessions(&mut app, false);
    let buffer = terminal.backend().buffer();
    assert!(!buffer[(0, 5)].modifier.contains(Modifier::BOLD));
    assert_eq!(buffer[(title, 0)].bg, FAR_BLUE);
    assert_eq!(buffer[(1, 1)].fg, palette::HOTKEY, "the heading");
    assert_eq!(buffer[(1, 8)].fg, palette::DATA, "the info line");
}

fn remote_app(dir: &TempDir) -> App {
    let entry = |name: &str, kind| DirEntry {
        name: name.to_owned(),
        kind,
        ..DirEntry::default()
    };
    let mut panel = RemotePanel::new(
        name("demo"),
        RemoteLink::new(Box::new(FakeRemoteFs::default())),
    );
    let shown = panel.show(
        RemotePath::parse("/var/www").expect("path"),
        vec![
            entry("html", EntryKind::Directory),
            entry("a.txt", EntryKind::File),
        ],
    );
    assert_eq!(shown.unusable_names, 0);
    let mut app = app_in(dir.path());
    app.left_panel.kind = PanelKind::Remote(Box::new(panel));
    app
}

#[test]
fn the_remote_panel_puts_each_part_on_its_row_and_marks_the_cursor() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = remote_app(&dir);
    if let PanelKind::Remote(panel) = &mut app.left_panel.kind {
        panel.move_cursor(RowMotion::Next);
    }
    let (rows, terminal) = draw_remote(&mut app, true);
    assert!(rows[0].contains(" demo:/var/www "), "{rows:?}");
    assert!(rows[1].starts_with("║Name"), "{rows:?}");
    assert!(rows[2].starts_with("║.. "), "{rows:?}");
    assert!(rows[3].starts_with("║html "), "{rows:?}");
    assert!(rows[4].starts_with("║a.txt "), "{rows:?}");
    assert!(rows[8].starts_with("║/var/www "), "{rows:?}");
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(1, 3)].bg, palette::CURSOR_BG);
    assert_ne!(buffer[(1, 2)].bg, palette::CURSOR_BG);
    assert_ne!(buffer[(1, 4)].bg, palette::CURSOR_BG);

    let (_, terminal) = draw_remote(&mut app, false);
    assert_ne!(terminal.backend().buffer()[(1, 3)].bg, palette::CURSOR_BG);
}
