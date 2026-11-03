//! A failed worker must not leave the viewer in a permanent loading state.

use super::*;
use std::time::{Duration, Instant};

#[test]
fn worker_panic_is_reported_to_the_open_viewer() {
    let mut app = App::new().expect("app");
    app.set_overlay(Overlay::Viewer(crate::models::Viewer::image(
        "broken.png".into(),
    )));
    let mut jobs = ImagePreviewJobs::with_decoder(|_| panic!("worker failed"));
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        jobs.pump(&mut app);
        if let Overlay::Viewer(viewer) = &app.overlay {
            if viewer.image == Some(ImageState::Failed("Image worker stopped".into())) {
                break;
            }
        }
        assert!(Instant::now() < until, "worker failure was not reported");
        std::thread::yield_now();
    }
}

#[test]
fn spawn_failure_and_fileless_previews_do_not_start_workers() {
    let mut app = App::new().expect("app");
    app.set_overlay(Overlay::Viewer(crate::models::Viewer::image(
        "spawn-fail.png".into(),
    )));
    let mut jobs = ImagePreviewJobs::with_decoder(|_| Err("unused".into()));
    FORCE_SPAWN_FAILURE.set(true);
    let redraw = jobs.pump(&mut app);
    FORCE_SPAWN_FAILURE.set(false);
    assert_eq!(redraw, Redraw::Needed);
    let Overlay::Viewer(viewer) = &app.overlay else {
        panic!("viewer");
    };
    assert_eq!(
        viewer.image,
        Some(ImageState::Failed("Cannot start image worker".into()))
    );

    let mut report = crate::models::Viewer::report("Report", vec!["ok".into()]);
    report.image = Some(ImageState::Pending);
    app.set_overlay(Overlay::Viewer(report));
    assert_eq!(jobs.pump(&mut app), Redraw::Unchanged);
    app.close_overlay();

    let qv = crate::models::QuickView {
        shown: None,
        preview: Preview::Image(ImageState::Pending),
        ..Default::default()
    };
    app.left_panel.kind = PanelKind::QuickView(Box::new(qv));
    assert_eq!(jobs.pump(&mut app), Redraw::Unchanged);
}

#[test]
fn closed_quickview_discards_in_flight_decoder_reply() {
    let mut app = App::new().expect("app");
    let (gate_tx, gate_rx) = mpsc::channel::<()>();
    let gate_rx = std::sync::Mutex::new(gate_rx);
    let mut jobs = ImagePreviewJobs::with_decoder(move |_| {
        gate_rx.lock().expect("lock").recv().expect("released");
        Err("done".into())
    });
    let qv = crate::models::QuickView {
        shown: Some("panel.png".into()),
        preview: Preview::Image(ImageState::Pending),
        ..Default::default()
    };
    app.left_panel.kind = PanelKind::QuickView(Box::new(qv));
    assert_eq!(jobs.pump(&mut app), Redraw::Needed);
    assert_eq!(jobs.pump(&mut app), Redraw::Unchanged);
    app.left_panel.kind = PanelKind::Files;
    gate_tx.send(()).expect("release");
    let until = Instant::now() + Duration::from_secs(5);
    while jobs.running.is_some() {
        assert_eq!(jobs.pump(&mut app), Redraw::Unchanged);
        assert!(Instant::now() < until);
        std::thread::yield_now();
    }
}
