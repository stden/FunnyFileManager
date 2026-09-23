//! Unit tests: a view mode lands on the active panel and says its name.

use super::set;
use crate::models::{App, PanelSide, PanelView};

#[test]
fn set_changes_the_active_panel_and_reports_the_mode() {
    let mut app = App::new().expect("test setup");
    app.active_panel = PanelSide::Right;
    set(&mut app, PanelView::Wide);
    assert_eq!(app.right_panel.view, PanelView::Wide);
    assert_eq!(
        app.left_panel.view,
        PanelView::Full,
        "the other is untouched"
    );
    assert!(app.status.to_string().contains(PanelView::Wide.label()));
}
