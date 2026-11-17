//! Background previews stay responsive and never publish stale images.

use super::*;
use crate::models::Panel;
use std::sync::Mutex;
use std::time::{Duration, Instant};

mod switching;

fn fixture() -> (tempfile::TempDir, App) {
    let dir = tempfile::tempdir().expect("directory");
    for name in ["a.png", "b.png", "c.png"] {
        std::fs::write(dir.path().join(name), b"image").expect("fixture");
    }
    let panel = Panel::new(dir.path().to_path_buf()).expect("panel");
    let mut app = App::with_panels(panel.clone(), panel);
    assert!(app.left_panel.select_named("a.png"));
    crate::ops::quickview::toggle(&mut app);
    (dir, app)
}

fn result(app: &App) -> Option<&str> {
    if let Overlay::Viewer(viewer) = &app.overlay {
        if let Some(ImageState::Failed(reason)) = &viewer.image {
            return Some(reason);
        }
    }
    let PanelKind::QuickView(view) = &app.right_panel.kind else {
        return None;
    };
    match &view.preview {
        Preview::Image(ImageState::Failed(reason)) => Some(reason),
        _ => None,
    }
}

fn pump_until(jobs: &mut ImagePreviewJobs, app: &mut App, expected: &str) {
    let until = Instant::now() + Duration::from_secs(5);
    while result(app) != Some(expected) {
        jobs.pump(app);
        assert!(Instant::now() < until, "preview never reached {expected}");
        std::thread::yield_now();
    }
}

fn select_each(app: &mut App, jobs: &mut ImagePreviewJobs, names: &[&str]) {
    for name in names {
        assert!(app.left_panel.select_named(name));
        crate::ops::quickview::refresh(app);
        jobs.pump(app);
    }
}

#[test]
fn scrolling_discards_obsolete_results_and_starts_only_the_latest_request() {
    let (_dir, mut app) = fixture();
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let release_rx = Arc::new(Mutex::new(release_rx));
    let mut jobs = ImagePreviewJobs::with_decoder(move |path| {
        let name = path
            .file_name()
            .expect("name")
            .to_string_lossy()
            .into_owned();
        started_tx.send(name.clone()).expect("receiver");
        if name == "a.png" {
            release_rx.lock().expect("lock").recv().expect("release");
        }
        Err(name)
    });

    jobs.pump(&mut app);
    assert_eq!(
        started_rx.recv_timeout(Duration::from_secs(5)).unwrap(),
        "a.png"
    );
    select_each(&mut app, &mut jobs, &["b.png", "c.png"]);
    assert!(started_rx.try_recv().is_err(), "only one worker may run");
    release_tx.send(()).expect("release worker");
    pump_until(&mut jobs, &mut app, "c.png");

    assert_eq!(
        started_rx.recv_timeout(Duration::from_secs(5)).unwrap(),
        "c.png"
    );
    assert!(started_rx.try_recv().is_err(), "b.png must never be queued");
}

#[test]
fn reopening_same_preview_does_not_accept_the_old_result() {
    let (_dir, mut app) = fixture();
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let release_rx = Arc::new(Mutex::new(release_rx));
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut jobs = ImagePreviewJobs::with_decoder(move |_| {
        let run = calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
        started_tx.send(run).expect("receiver");
        if run == 1 {
            release_rx.lock().expect("lock").recv().expect("release");
        }
        Err(format!("run-{run}"))
    });

    jobs.pump(&mut app);
    assert_eq!(started_rx.recv_timeout(Duration::from_secs(5)).unwrap(), 1);
    crate::ops::quickview::toggle(&mut app);
    crate::ops::quickview::toggle(&mut app);
    release_tx.send(()).expect("release");
    pump_until(&mut jobs, &mut app, "run-2");
    assert_eq!(started_rx.recv_timeout(Duration::from_secs(5)).unwrap(), 2);
}

#[test]
fn a_real_png_is_decoded_and_published_to_the_panel() {
    let (dir, mut app) = fixture();
    image::RgbaImage::from_pixel(2, 1, image::Rgba([10, 20, 30, 255]))
        .save(dir.path().join("a.png"))
        .expect("write PNG");
    let mut jobs = ImagePreviewJobs::default();
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        jobs.pump(&mut app);
        if let PanelKind::QuickView(view) = &app.right_panel.kind {
            if let Preview::Image(ImageState::Ready(image)) = &view.preview {
                assert_eq!((image.width(), image.height()), (2, 1));
                assert_eq!(image.format(), "PNG");
                break;
            }
        }
        assert!(Instant::now() < until, "PNG preview was not published");
        std::thread::yield_now();
    }
}

#[test]
fn full_screen_image_is_loaded_before_a_pending_quick_preview() {
    let (dir, mut app) = fixture();
    app.set_overlay(Overlay::Viewer(crate::models::Viewer::image(
        dir.path().join("c.png"),
    )));
    let (started_tx, started_rx) = mpsc::channel();
    let mut jobs = ImagePreviewJobs::with_decoder(move |path| {
        let name = path
            .file_name()
            .expect("name")
            .to_string_lossy()
            .into_owned();
        started_tx.send(name.clone()).expect("receiver");
        Err(name)
    });
    jobs.pump(&mut app);
    assert_eq!(
        started_rx.recv_timeout(Duration::from_secs(5)).unwrap(),
        "c.png"
    );
    pump_until(&mut jobs, &mut app, "c.png");
    app.set_overlay(Overlay::None);
    pump_until(&mut jobs, &mut app, "a.png");
    assert_eq!(
        started_rx.recv_timeout(Duration::from_secs(5)).unwrap(),
        "a.png"
    );
}

#[test]
fn closing_the_viewer_discards_its_in_flight_result() {
    let (dir, mut app) = fixture();
    app.right_panel.kind = PanelKind::Files;
    app.set_overlay(Overlay::Viewer(crate::models::Viewer::image(
        dir.path().join("a.png"),
    )));
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let release_rx = Arc::new(Mutex::new(release_rx));
    let mut jobs = ImagePreviewJobs::with_decoder(move |_| {
        started_tx.send(()).expect("receiver");
        release_rx.lock().expect("lock").recv().expect("release");
        Err("obsolete".to_owned())
    });
    jobs.pump(&mut app);
    started_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("worker started");
    app.set_overlay(Overlay::None);
    release_tx.send(()).expect("release");
    let until = Instant::now() + Duration::from_secs(5);
    while jobs.running.is_some() {
        jobs.pump(&mut app);
        assert!(Instant::now() < until, "worker did not finish");
        std::thread::yield_now();
    }
    assert!(matches!(app.overlay, Overlay::None));
    assert!(matches!(app.right_panel.kind, PanelKind::Files));
}
