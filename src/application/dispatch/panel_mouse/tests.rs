//! Panel click routing respects hidden panels, quick views, and tree ownership.

use super::*;
use crate::models::{PanelView, PanelsShown};
use crate::WritableTestBackend;

fn app_in(dir: &tempfile::TempDir) -> App {
    let panel = Panel::from_entries(dir.path().to_path_buf(), Vec::new());
    App::with_panels(panel.clone(), panel)
}

#[test]
fn a_row_on_a_panel_that_is_not_displayed_is_not_a_target() {
    let dir = tempfile::TempDir::new().expect("temporary directory");
    let mut app = app_in(&dir);
    app.right_panel.view = PanelView::Brief;
    app.panels_shown = PanelsShown::LeftOnly;
    let row = Target::PanelRow {
        side: PanelSide::Right,
        row: 2,
    };
    assert_eq!(
        target(&app, row, Rect::new(0, 0, 80, 24), 60),
        Target::Nothing
    );
}

#[test]
fn clicking_quick_view_breaks_a_pending_double_click() {
    let dir = tempfile::TempDir::new().expect("temporary directory");
    let mut app = app_in(&dir);
    app.left_panel.kind = PanelKind::QuickView(Box::default());
    let mut terminal = Terminal::new(WritableTestBackend::new(80, 24)).expect("terminal");
    let mut clicks = PanelClicks::default();
    let now = Instant::now();
    assert_eq!(clicks.press(PanelSide::Left, 0, now), PanelRowClick::Select);
    click_row(&mut terminal, &mut app, PanelSide::Left, 0, &mut clicks);
    assert!(matches!(app.left_panel.kind, PanelKind::QuickView(_)));
    assert_eq!(clicks.press(PanelSide::Left, 0, now), PanelRowClick::Select);
}

#[test]
fn tree_row_handler_does_not_navigate_a_file_panel() {
    let dir = tempfile::TempDir::new().expect("temporary directory");
    let mut app = app_in(&dir);
    let mut terminal = Terminal::new(WritableTestBackend::new(80, 24)).expect("terminal");
    click_tree_row(
        &mut terminal,
        &mut app,
        PanelSide::Left,
        0,
        &mut PanelClicks::default(),
    );
    assert!(matches!(app.left_panel.kind, PanelKind::Files));
    assert_eq!(app.left_panel.path, dir.path());
}
