//! Unit tests: what `App` takes over from the loaded settings.

use super::{App, PanelSide, PanelView, PanelsShown};

#[test]
fn saved_description_names_reach_both_panels() {
    let mut app = App::new().expect("test setup");
    let names = vec!["Descript.ion".to_owned(), "notes.txt".to_owned()];
    app.config.descriptions.names.clone_from(&names);

    app.adopt_description_names();

    assert_eq!(app.left_panel.description_names, names);
    assert_eq!(app.right_panel.description_names, names);
}

#[test]
fn the_panels_on_screen_fold_a_fullscreen_view_into_panels_shown() {
    let mut app = App::new().expect("test setup");
    assert_eq!(app.panels_on_screen(), PanelsShown::Both);
    app.active_panel = PanelSide::Right;
    app.right_panel.view = PanelView::Detailed;
    assert_eq!(app.panels_on_screen(), PanelsShown::RightOnly);
}
