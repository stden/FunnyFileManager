//! Hidden quick views must not schedule image work or retain rendered frames.

use super::*;
use crate::models::{Panel, PanelsShown, QuickView, ViewMode, Viewer};
use ratatui::{backend::TestBackend, Terminal};
use std::time::{Duration, Instant};

fn fixture() -> (tempfile::TempDir, App) {
    let dir = tempfile::tempdir().expect("directory");
    std::fs::write(dir.path().join("photo.png"), b"image").expect("fixture");
    let panel = Panel::new(dir.path().to_path_buf()).expect("panel");
    let mut app = App::with_panels(panel.clone(), panel);
    assert!(app.left_panel.select_named("photo.png"));
    crate::ops::quickview::toggle(&mut app);
    (dir, app)
}

fn red_image() -> DecodedImage {
    DecodedImage {
        width: 1,
        height: 1,
        format: "PNG",
        size: 4,
        pixels: Arc::from([255, 0, 0, 255]),
    }
}

fn quick_state(app: &App) -> &ImageState {
    let PanelKind::QuickView(view) = &app.right_panel.kind else {
        panic!("quick view expected")
    };
    let Preview::Image(image) = &view.preview else {
        panic!("image expected")
    };
    image
}

fn wait_for_ready(jobs: &mut ImagePreviewJobs, app: &mut App) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        jobs.refresh(app);
        if matches!(quick_state(app), ImageState::Ready(_)) {
            return;
        }
        assert!(Instant::now() < deadline, "image never loaded");
        std::thread::yield_now();
    }
}

fn cache_image(image: &DecodedImage) {
    let mut terminal = Terminal::new(TestBackend::new(8, 4)).expect("terminal");
    terminal
        .draw(|frame| crate::ui::image::render(frame, frame.area(), image, 1, (0, 0)))
        .expect("draw");
    let deadline = Instant::now() + Duration::from_secs(5);
    while !crate::ui::image::pump() {
        assert!(Instant::now() < deadline, "renderer did not finish");
        std::thread::yield_now();
    }
}

#[test]
fn hidden_pending_preview_starts_only_after_showing_the_panel() {
    let (_dir, mut app) = fixture();
    app.panels_shown = PanelsShown::LeftOnly;
    let mut jobs = ImagePreviewJobs::with_decoder(|_| Ok(red_image()));
    jobs.refresh(&mut app);
    assert!(jobs.running.is_none(), "hidden preview must not load");
    assert!(matches!(quick_state(&app), ImageState::Pending));

    app.panels_shown = PanelsShown::Both;
    wait_for_ready(&mut jobs, &mut app);
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).expect("terminal");
    terminal
        .draw(|frame| crate::ui::ui(frame, &mut app))
        .expect("draw");
    assert!(crate::test_support::buffer_string(terminal.backend().buffer()).contains("PNG"));
}

#[test]
fn console_hides_panel_jobs_but_not_a_fullscreen_viewer() {
    let (dir, mut app) = fixture();
    app.view = ViewMode::Console;
    let mut jobs = ImagePreviewJobs::with_decoder(|_| Ok(red_image()));
    jobs.refresh(&mut app);
    assert!(jobs.running.is_none(), "console hides panel previews");

    app.set_overlay(Overlay::Viewer(Viewer::image(dir.path().join("photo.png"))));
    jobs.refresh(&mut app);
    assert!(matches!(
        jobs.running.as_ref().map(|job| job.target),
        Some(Target::Viewer)
    ));
}

#[test]
fn fullscreen_panel_and_text_viewer_do_not_start_hidden_preview() {
    let (_dir, mut app) = fixture();
    app.left_panel.view = crate::models::PanelView::Detailed;
    let mut jobs = ImagePreviewJobs::with_decoder(|_| Ok(red_image()));
    jobs.refresh(&mut app);
    assert!(
        jobs.running.is_none(),
        "fullscreen left panel hides the right"
    );

    app.left_panel.view = crate::models::PanelView::Full;
    app.set_overlay(Overlay::Viewer(Viewer::report("report", Vec::new())));
    jobs.refresh(&mut app);
    assert!(
        jobs.running.is_none(),
        "fullscreen text viewer hides both panels"
    );
    app.close_overlay();
    wait_for_ready(&mut jobs, &mut app);
}

#[test]
fn hiding_a_ready_quick_view_releases_its_renderer_cache() {
    crate::ui::image::clear();
    let (dir, mut app) = fixture();
    let pixels: Arc<[u8]> = Arc::from([255, 0, 0, 255]);
    let mut image = red_image();
    image.pixels = Arc::clone(&pixels);
    let mut quick = QuickView::default();
    quick.show(
        Some(dir.path().join("photo.png")),
        Preview::Image(ImageState::Ready(image.clone())),
    );
    app.right_panel.kind = PanelKind::QuickView(Box::new(quick));
    let mut jobs = ImagePreviewJobs::default();
    jobs.refresh(&mut app);
    cache_image(&image);
    drop(image);
    assert_eq!(
        Arc::strong_count(&pixels),
        3,
        "preview and cache retain image"
    );

    app.panels_shown = PanelsShown::LeftOnly;
    jobs.refresh(&mut app);
    assert_eq!(
        Arc::strong_count(&pixels),
        2,
        "hidden renderer cache is released"
    );
    assert!(matches!(quick_state(&app), ImageState::Ready(_)));
}
