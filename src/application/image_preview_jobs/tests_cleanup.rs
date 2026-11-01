//! The last closed image view releases the shared renderer's cached pixels.

use super::*;
use crate::models::{Panel, QuickView, Viewer};
use ratatui::{backend::TestBackend, Terminal};
use std::time::{Duration, Instant};

fn image(pixels: &Arc<[u8]>) -> DecodedImage {
    DecodedImage {
        width: 1,
        height: 1,
        format: "PNG",
        size: 4,
        pixels: Arc::clone(pixels),
    }
}

fn cache_image(image: &DecodedImage) {
    let mut terminal = Terminal::new(TestBackend::new(40, 10)).expect("terminal");
    terminal
        .draw(|frame| crate::ui::image::render(frame, frame.area(), image, 1, (0, 0)))
        .expect("draw");
    let until = Instant::now() + Duration::from_secs(5);
    while !crate::ui::image::pump() {
        assert!(Instant::now() < until, "image was not prepared");
        std::thread::yield_now();
    }
}

fn show_viewer(app: &mut App, path: PathBuf, image: &DecodedImage) {
    let mut viewer = Viewer::image(path);
    viewer.image = Some(ImageState::Ready(image.clone()));
    app.set_overlay(Overlay::Viewer(viewer));
}

fn show_quick_view(app: &mut App, path: PathBuf, image: &DecodedImage) {
    let mut quick = QuickView::default();
    quick.show(Some(path), Preview::Image(ImageState::Ready(image.clone())));
    app.right_panel.kind = PanelKind::QuickView(Box::new(quick));
}

fn quick_fixture() -> (tempfile::TempDir, App, Arc<[u8]>, PathBuf) {
    let dir = tempfile::tempdir().expect("directory");
    let path = dir.path().join("image.png");
    std::fs::write(&path, b"fixture").expect("image fixture");
    let panel = Panel::new(dir.path().to_path_buf()).expect("panel");
    let mut app = App::with_panels(panel.clone(), panel);
    assert!(app.left_panel.select_named("image.png"));
    let pixels = Arc::from([255, 0, 0, 255]);
    show_quick_view(&mut app, path.clone(), &image(&pixels));
    (dir, app, pixels, path)
}

#[test]
fn closing_the_last_image_view_releases_cached_pixels() {
    crate::ui::image::clear();
    let dir = tempfile::tempdir().expect("directory");
    let panel = Panel::new(dir.path().to_path_buf()).expect("panel");
    let mut app = App::with_panels(panel.clone(), panel);
    let pixels: Arc<[u8]> = Arc::from([255, 0, 0, 255]);
    let image = image(&pixels);
    show_viewer(&mut app, dir.path().join("image.png"), &image);
    let mut jobs = ImagePreviewJobs::default();
    jobs.refresh(&mut app);
    cache_image(&image);
    drop(image);
    app.close_overlay();
    jobs.refresh(&mut app);

    assert_eq!(
        Arc::strong_count(&pixels),
        1,
        "closed image is still cached"
    );
}

#[test]
fn closing_the_viewer_keeps_the_cache_while_quick_view_is_open() {
    crate::ui::image::clear();
    let dir = tempfile::tempdir().expect("directory");
    let path = dir.path().join("image.png");
    std::fs::write(&path, b"fixture").expect("image fixture");
    let panel = Panel::new(dir.path().to_path_buf()).expect("panel");
    let mut app = App::with_panels(panel.clone(), panel);
    assert!(app.left_panel.select_named("image.png"));
    let pixels: Arc<[u8]> = Arc::from([255, 0, 0, 255]);
    let image = image(&pixels);
    show_quick_view(&mut app, path.clone(), &image);
    show_viewer(&mut app, path, &image);
    let mut jobs = ImagePreviewJobs::default();
    jobs.refresh(&mut app);
    cache_image(&image);
    drop(image);
    app.close_overlay();
    jobs.refresh(&mut app);
    assert_eq!(
        Arc::strong_count(&pixels),
        3,
        "preview lost its cached image"
    );
    app.right_panel.kind = PanelKind::Files;
    jobs.refresh(&mut app);
    assert_eq!(
        Arc::strong_count(&pixels),
        1,
        "closed image is still cached"
    );
}

#[test]
fn switching_to_console_releases_the_hidden_preview_cache() {
    crate::ui::image::clear();
    let (_dir, mut app, pixels, _path) = quick_fixture();
    let image = image(&pixels);
    let mut jobs = ImagePreviewJobs::default();
    jobs.refresh(&mut app);
    cache_image(&image);
    drop(image);
    assert_eq!(Arc::strong_count(&pixels), 3);

    app.view = crate::models::ViewMode::Console;
    jobs.refresh(&mut app);
    assert_eq!(Arc::strong_count(&pixels), 2, "console releases the cache");
    assert!(
        matches!(&app.right_panel.kind, PanelKind::QuickView(view) if matches!(view.preview, Preview::Image(ImageState::Ready(_))))
    );
}

#[test]
fn hiding_a_quick_view_does_not_clear_a_visible_fullscreen_image() {
    crate::ui::image::clear();
    let (_dir, mut app, pixels, path) = quick_fixture();
    let image = image(&pixels);
    show_viewer(&mut app, path, &image);
    let mut jobs = ImagePreviewJobs::default();
    jobs.refresh(&mut app);
    cache_image(&image);
    drop(image);
    assert_eq!(Arc::strong_count(&pixels), 4);

    app.panels_shown = crate::models::PanelsShown::LeftOnly;
    app.view = crate::models::ViewMode::Console;
    jobs.refresh(&mut app);
    assert_eq!(Arc::strong_count(&pixels), 4, "viewer stays visible");
    app.close_overlay();
    jobs.refresh(&mut app);
    assert_eq!(
        Arc::strong_count(&pixels),
        2,
        "last visible view releases cache"
    );
}
