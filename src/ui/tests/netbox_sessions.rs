//! Unit tests: the `NetBox` sessions panel draws what its state says — its
//! title, heading, rows, cursor bar and count — scrolls to keep the cursor on
//! screen, and always offers the way back.

use ratatui::Terminal;
use tempfile::TempDir;

use crate::models::{App, PanelKind, RowMotion, SessionEdit, SessionsPanel};
use crate::sessions::SessionStore;
use crate::test_support::netbox::{app_in, app_on_sessions, name, session};
use crate::test_support::{buffer_text, WritableTestBackend};
use crate::ui::palette;
use crate::ui::{render_overlays, ui};

pub(super) fn terminal(width: u16, height: u16) -> Terminal<WritableTestBackend> {
    Terminal::new(WritableTestBackend::new(width, height)).expect("test terminal")
}

pub(super) fn draw(
    app: &mut App,
    width: u16,
    height: u16,
) -> (String, Terminal<WritableTestBackend>) {
    let mut terminal = terminal(width, height);
    terminal.draw(|f| ui(f, app)).expect("draw");
    (buffer_text(&terminal), terminal)
}

pub(super) fn draw_overlay(app: &mut App) -> (String, Terminal<WritableTestBackend>) {
    let mut terminal = terminal(80, 24);
    terminal.draw(|f| render_overlays(f, app)).expect("draw");
    (buffer_text(&terminal), terminal)
}

/// The row of the screen holding `text`, and the column it starts in.
pub(super) fn find(screen: &str, text: &str) -> (u16, u16) {
    screen
        .lines()
        .enumerate()
        .find_map(|(row, line)| {
            let (before, _) = line.split_once(text)?;
            Some((
                u16::try_from(before.chars().count()).expect("fits"),
                u16::try_from(row).expect("fits"),
            ))
        })
        .unwrap_or_else(|| panic!("{text:?} is on screen:\n{screen}"))
}

pub(super) fn sessions_app(dir: &TempDir) -> App {
    let mut store = SessionStore::default();
    store.upsert(session("a.test", Some("demo")));
    let mut renamed = session("b.test", Some("demo"));
    renamed.name = name("web");
    store.upsert(renamed);
    app_on_sessions(dir.path(), store)
}

/// The Shift+F4 dialog with `host` typed in.
pub(super) fn dialog_for(host: &str) -> SessionEdit {
    let mut dialog = SessionEdit::create();
    dialog.host = host.to_owned();
    dialog
}

#[test]
fn the_sessions_panel_draws_its_title_heading_rows_and_count() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = sessions_app(&dir);
    let (screen, _) = draw(&mut app, 100, 20);
    for text in [
        " NetBox: Stored sessions [/] ",
        "Session name",
        "..",
        "demo@a.test",
        "web  demo@b.test",
        "2 sessions",
    ] {
        assert!(screen.contains(text), "{text:?} in\n{screen}");
    }

    let mut single = app_on_sessions(dir.path(), {
        let mut store = SessionStore::default();
        store.upsert(session("a.test", None));
        store
    });
    let (screen, _) = draw(&mut single, 100, 20);
    assert!(screen.contains("1 session "), "{screen}");
}

#[test]
fn the_cursor_bar_marks_the_active_panels_row_only() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = sessions_app(&dir);
    if let PanelKind::Sessions(panel) = &mut app.left_panel.kind {
        panel.move_cursor(RowMotion::Next);
    }
    let (screen, terminal) = draw(&mut app, 100, 20);
    let (x, y) = find(&screen, "demo@a.test");
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(x, y)].bg, palette::CURSOR_BG);
    let (x, y) = find(&screen, "web  demo");
    assert_ne!(buffer[(x, y)].bg, palette::CURSOR_BG);

    app.active_panel = crate::models::PanelSide::Right;
    let (screen, terminal) = draw(&mut app, 100, 20);
    let (x, y) = find(&screen, "demo@a.test");
    assert_ne!(
        terminal.backend().buffer()[(x, y)].bg,
        palette::CURSOR_BG,
        "no bar on the inactive panel"
    );
}

#[test]
fn a_long_list_scrolls_to_keep_the_cursor_on_screen() {
    let dir = TempDir::new().expect("temp dir");
    let mut store = SessionStore::default();
    for index in 0..30 {
        store.upsert(session(&format!("h{index:02}.test"), None));
    }
    let mut app = app_on_sessions(dir.path(), store);
    if let PanelKind::Sessions(panel) = &mut app.left_panel.kind {
        panel.move_cursor(RowMotion::Last);
    }
    let (screen, _) = draw(&mut app, 100, 16);
    assert!(screen.contains("h29.test"), "{screen}");
    assert!(!screen.contains("h00.test"), "{screen}");
    let PanelKind::Sessions(panel) = &app.left_panel.kind else {
        panic!("the sessions list");
    };
    assert!(panel.scroll > 0, "the scroll is kept for the next frame");
}

#[test]
fn a_panel_too_short_for_a_row_draws_only_its_frame() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = sessions_app(&dir);
    let mut terminal = terminal(60, 4);
    let area = ratatui::layout::Rect::new(0, 0, 60, 4);
    terminal
        .draw(|f| super::sessions::render(f, &mut app.left_panel, area, true))
        .expect("draw");
    let screen = buffer_text(&terminal);
    assert!(screen.contains("NetBox: Stored sessions"), "{screen}");
    assert!(!screen.contains("Session name"), "{screen}");
}

#[test]
fn an_empty_sessions_panel_still_offers_the_way_back() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    app.left_panel.kind = PanelKind::Sessions(Box::new(SessionsPanel::from_store(
        &SessionStore::default(),
    )));
    let (screen, _) = draw(&mut app, 80, 20);
    assert!(screen.contains(".."), "{screen}");
    assert!(screen.contains("0 sessions"), "{screen}");
}
