//! Results from an obsolete preview must never replace the current image.
use super::*;
use crate::models::Viewer;

fn in_flight(
    target: Target,
    path: PathBuf,
) -> (ImagePreviewJobs, mpsc::Sender<Result<DecodedImage, String>>) {
    let (sender, receiver) = mpsc::channel();
    let mut jobs = ImagePreviewJobs::with_decoder(|_| Err("current".to_owned()));
    jobs.running = Some(Running {
        id: 1,
        target,
        path,
        receiver,
    });
    jobs.next_id = 1;
    (jobs, sender)
}

#[test]
fn opening_another_viewer_discards_the_previous_viewers_result() {
    let (dir, mut app) = fixture();
    let old = dir.path().join("a.png");
    app.set_overlay(Overlay::Viewer(Viewer::image(old.clone())));
    let (mut jobs, sender) = in_flight(Target::Viewer, old);
    app.set_overlay(Overlay::Viewer(Viewer::image(dir.path().join("b.png"))));
    sender.send(Err("obsolete".to_owned())).unwrap();
    assert_eq!(jobs.collect(&mut app), Redraw::Unchanged);
    assert!(
        matches!(&app.overlay, Overlay::Viewer(viewer) if viewer.image == Some(ImageState::Pending))
    );
    pump_until(&mut jobs, &mut app, "current");
    assert_eq!(result(&app), Some("current"));
}

#[test]
fn selecting_a_different_image_discards_the_old_preview_result() {
    let (dir, mut app) = fixture();
    let (mut jobs, sender) = in_flight(Target::Panel(PanelSide::Right), dir.path().join("a.png"));
    assert!(app.left_panel.select_named("b.png"));
    crate::ops::quickview::refresh(&mut app);
    sender.send(Err("obsolete".to_owned())).unwrap();
    assert_eq!(jobs.collect(&mut app), Redraw::Unchanged);
    let PanelKind::QuickView(view) = &app.right_panel.kind else {
        panic!("quick view")
    };
    assert_eq!(
        view.shown.as_deref(),
        Some(dir.path().join("b.png").as_path())
    );
    assert_eq!(view.preview, Preview::Image(ImageState::Pending));
    pump_until(&mut jobs, &mut app, "current");
}

#[test]
fn replacing_an_image_with_text_keeps_the_new_preview() {
    let (dir, mut app) = fixture();
    let path = dir.path().join("a.png");
    let (mut jobs, sender) = in_flight(Target::Panel(PanelSide::Right), path.clone());
    let PanelKind::QuickView(view) = &mut app.right_panel.kind else {
        panic!("quick view")
    };
    let text = crate::models::TextPreview::decoded(&path, b"replacement text");
    view.show(Some(path), Preview::Text(text));
    sender.send(Err("obsolete".to_owned())).unwrap();
    assert_eq!(jobs.collect(&mut app), Redraw::Unchanged);
    let PanelKind::QuickView(view) = &app.right_panel.kind else {
        panic!("quick view")
    };
    assert!(matches!(&view.preview, Preview::Text(text) if text.lines == ["replacement text"]));
}
