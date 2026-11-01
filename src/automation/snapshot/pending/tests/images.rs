//! Purpose: Wait for decoders that can run, without waiting forever for a hidden pending preview.

use super::*;
use crate::models::{Editor, PanelsShown, QuickView, Viewer};

fn preview(app: &mut App, state: ImageState) {
    app.right_panel.kind = PanelKind::QuickView(Box::new(QuickView {
        shown: Some(app.right_panel.path.join("fixture.png")),
        preview: Preview::Image(state),
        ..QuickView::default()
    }));
}

#[test]
fn full_screen_image_waits_for_pending_and_loading_then_stops_after_failure() {
    let root = TempDir::new().unwrap();
    let mut app = application(&root);
    let mut viewer = Viewer::image(root.path().join("fixture.png"));
    for (image, pending) in [
        (ImageState::Pending, true),
        (ImageState::Loading(2), true),
        (ImageState::Failed("fixture".into()), false),
    ] {
        viewer.image = Some(image);
        app.overlay = Overlay::Viewer(viewer.clone());
        assert_eq!(is_pending(&app), pending);
    }
}

#[test]
fn hidden_or_obscured_pending_previews_do_not_create_an_infinite_wait() {
    let root = TempDir::new().unwrap();
    let mut app = application(&root);
    preview(&mut app, ImageState::Pending);
    assert!(is_pending(&app));
    app.panels_shown = PanelsShown::LeftOnly;
    assert!(!is_pending(&app));
    app.panels_shown = PanelsShown::Both;
    app.view = ViewMode::Console;
    assert!(!is_pending(&app));
    app.view = ViewMode::Panels;
    app.overlay = Overlay::Editor(Editor::default());
    assert!(!is_pending(&app));
    app.overlay = Overlay::Viewer(Viewer::report("text", Vec::new()));
    assert!(!is_pending(&app));
}

#[test]
fn already_loading_previews_finish_even_when_their_panel_becomes_hidden() {
    let root = TempDir::new().unwrap();
    let mut app = application(&root);
    preview(&mut app, ImageState::Loading(3));
    app.panels_shown = PanelsShown::LeftOnly;
    assert!(is_pending(&app));
    preview(&mut app, ImageState::Failed("fixture".into()));
    assert!(!is_pending(&app));
}

#[test]
fn images_without_a_source_path_are_not_queued_by_the_decoder() {
    let root = TempDir::new().unwrap();
    let mut app = application(&root);
    app.right_panel.kind = PanelKind::QuickView(Box::new(QuickView {
        preview: Preview::Image(ImageState::Pending),
        ..QuickView::default()
    }));
    assert!(!is_pending(&app));
}

#[test]
fn text_previews_do_not_keep_automation_wait_pending() {
    let root = TempDir::new().unwrap();
    let mut app = application(&root);
    let path = root.path().join("fixture.txt");
    app.right_panel.kind = PanelKind::QuickView(Box::new(QuickView {
        shown: Some(path.clone()),
        preview: Preview::Text(crate::models::TextPreview::decoded(&path, b"ready text")),
        ..QuickView::default()
    }));
    assert!(!is_pending(&app));
}
