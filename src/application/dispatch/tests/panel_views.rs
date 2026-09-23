//! The arrows and the mouse in Far's multi-column views.
//!
//! An 80x24 screen leaves every panel 16 entry rows, so one Brief column is
//! 16 entries and the next column starts at entry 16.

use std::fs;

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use tempfile::TempDir;

use super::*;
use crate::models::{Panel, PanelSide, PanelView, PanelsShown};

const ROWS_PER_COLUMN: usize = 16;

fn brief_app() -> (App, Terminal<WritableTestBackend>, TempDir) {
    let dir = TempDir::new().expect("test setup");
    for index in 0..45 {
        fs::write(dir.path().join(format!("file{index:02}")), b"x").expect("test setup");
    }
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(dir.path().to_path_buf()).expect("test setup");
    app.left_panel.view = PanelView::Brief;
    app.left_panel.select(Some(0));
    app.active_panel = PanelSide::Left;
    let terminal = Terminal::new(WritableTestBackend::new(80, 24)).expect("test setup");
    (app, terminal, dir)
}

#[test]
fn left_and_right_move_a_column_in_a_multi_column_view() {
    let (mut app, mut terminal, _dir) = brief_app();
    let mut press = |app: &mut App, code| dispatch_key(&mut terminal, app, key_event(code));

    press(&mut app, KeyCode::Right);
    assert_eq!(app.left_panel.selected(), Some(ROWS_PER_COLUMN));
    press(&mut app, KeyCode::Right);
    assert_eq!(app.left_panel.selected(), Some(2 * ROWS_PER_COLUMN));
    press(&mut app, KeyCode::Left);
    press(&mut app, KeyCode::Left);
    press(&mut app, KeyCode::Left);
    assert_eq!(
        app.left_panel.selected(),
        Some(0),
        "stops on the first entry"
    );
    assert_eq!(app.active_panel, PanelSide::Left, "the panel stays active");
}

#[test]
fn left_and_right_still_choose_the_panel_in_a_single_column_view() {
    let (mut app, mut terminal, _dir) = brief_app();
    app.left_panel.view = PanelView::Wide;
    dispatch_key(&mut terminal, &mut app, key_event(KeyCode::Right));
    assert_eq!(app.active_panel, PanelSide::Right);
    assert_eq!(app.left_panel.selected(), Some(0));
}

/// With the other panel hidden the Brief panel spans all 80 columns: 78
/// interior cells give columns of 25, 25 and 26, so screen column 30 is in
/// the second one — not the third, which halving the screen would say.
#[test]
fn a_click_on_a_lone_brief_panel_reads_the_whole_screen_width() {
    let (mut app, mut terminal, _dir) = brief_app();
    app.panels_shown = PanelsShown::LeftOnly;
    let mouse = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 30,
        row: 3,
        modifiers: KeyModifiers::NONE,
    };
    dispatch_mouse(
        &mut terminal,
        &mut app,
        mouse,
        Rect::new(0, 0, 80, 24),
        &mut crate::mouse::PanelClicks::default(),
    );
    assert_eq!(app.left_panel.selected(), Some(ROWS_PER_COLUMN));
}
