//! Purpose: clicks on the rows of a `NetBox` panel move that panel's cursor,
//! never the file list hidden beneath it, and a double click is Enter.

use super::*;
use crate::models::{PanelKind, RowMotion, SessionsPanel};
use crate::test_support::netbox::{app_on_host, session};

fn on_sessions(app: &mut App) {
    app.sessions.upsert(session("a.test", None));
    app.sessions.upsert(session("b.test", None));
    app.left_panel.kind = PanelKind::Sessions(Box::new(SessionsPanel::from_store(&app.sessions)));
}

fn sessions_cursor(app: &App) -> usize {
    let PanelKind::Sessions(panel) = &app.left_panel.kind else {
        panic!("the sessions list");
    };
    panel.cursor()
}

#[test]
fn a_click_on_a_session_moves_the_lists_cursor_and_not_the_files() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&["a.txt", "b.txt"]);
    on_sessions(&mut app);
    let files_cursor = app.left_panel.selected();
    terminal.draw(|f| ui(f, &mut app)).expect("draw");
    let area = terminal.get_frame().area();
    let column = area.x + 10;
    let second = panel_row_at(area, column, |row| row == 2);
    let mut clicks = crate::mouse::PanelClicks::default();

    click_at(&mut terminal, &mut app, (column, second), &mut clicks);
    assert_eq!(sessions_cursor(&app), 2);
    assert_eq!(
        app.left_panel.selected(),
        files_cursor,
        "the hidden list is untouched"
    );

    click_at(&mut terminal, &mut app, (column, second), &mut clicks);
    assert!(
        app.status_text().contains("this build cannot connect"),
        "a double click is Enter on the session: {}",
        app.status_text()
    );
}

#[test]
fn a_click_below_the_last_session_selects_nothing_and_breaks_a_double_click() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&[]);
    on_sessions(&mut app);
    terminal.draw(|f| ui(f, &mut app)).expect("draw");
    let area = terminal.get_frame().area();
    let column = area.x + 10;
    let parent = panel_row_at(area, column, |row| row == 0);
    let blank = panel_row_at(area, column, |row| row == 5);
    let mut clicks = crate::mouse::PanelClicks::default();

    for row in [parent, blank, parent] {
        click_at(&mut terminal, &mut app, (column, row), &mut clicks);
    }
    assert_eq!(sessions_cursor(&app), 0);
    assert!(
        app.left_panel.kind.is_netbox(),
        "no Enter on `..`: it was not a double click"
    );
}

#[test]
fn a_double_click_on_a_remote_directory_enters_it() {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let mut app = app_on_host(dir.path(), &[("/etc/hostname", 5)], None);
    if let PanelKind::Remote(panel) = &mut app.left_panel.kind {
        panel.move_cursor(RowMotion::First);
    }
    let mut terminal = Terminal::new(WritableTestBackend::new(120, 30)).expect("terminal");
    terminal.draw(|f| ui(f, &mut app)).expect("draw");
    let area = terminal.get_frame().area();
    let column = area.x + 10;
    let etc = panel_row_at(area, column, |row| row == 1);
    let mut clicks = crate::mouse::PanelClicks::default();
    click_at(&mut terminal, &mut app, (column, etc), &mut clicks);
    click_at(&mut terminal, &mut app, (column, etc), &mut clicks);
    let PanelKind::Remote(panel) = &app.left_panel.kind else {
        panic!("still on the host");
    };
    assert_eq!(panel.path().as_str(), "/etc");
}
