//! Image viewer and quick-view steps using real files and the decoder worker.

use std::time::{Duration, Instant};

use cucumber::{given, then, when};

use super::BehaviorWorld;
use crate::{
    application::image_preview_jobs::ImagePreviewJobs,
    models::{App, ImageState, Overlay, PanelKind, Preview},
};

#[given(expr = "a PNG image named {string} with size {int} by {int}")]
fn png(world: &mut BehaviorWorld, name: String, width: u32, height: u32) {
    let case = world.case();
    image::RgbaImage::from_pixel(width, height, image::Rgba([10, 20, 30, 255]))
        .save(case.root.path().join("left").join(&name))
        .unwrap();
    select_image(world, &name);
}

#[given(expr = "a corrupt PNG image named {string}")]
fn corrupt_png(world: &mut BehaviorWorld, name: String) {
    let case = world.case();
    std::fs::write(case.root.path().join("left").join(&name), b"not a PNG").unwrap();
    select_image(world, &name);
}

fn select_image(world: &mut BehaviorWorld, name: &str) {
    let case = world.case();
    case.app.left_panel.load_entries().unwrap();
    assert!(case.app.left_panel.select_named(name));
    case.paint();
}

#[when("the image preview finishes loading")]
fn load_image(world: &mut BehaviorWorld) {
    let case = world.case();
    let mut jobs = ImagePreviewJobs::default();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        jobs.refresh(&mut case.app);
        if image_finished(&case.app) {
            case.paint();
            return;
        }
        assert!(Instant::now() < deadline, "image preview did not finish");
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn image_finished(app: &App) -> bool {
    let state = match &app.overlay {
        Overlay::Viewer(viewer) => viewer.image.as_ref(),
        _ => match &app.right_panel.kind {
            PanelKind::QuickView(view) => match &view.preview {
                Preview::Image(state) => Some(state),
                _ => None,
            },
            _ => None,
        },
    };
    matches!(state, Some(ImageState::Ready(_) | ImageState::Failed(_)))
}

#[then("the image is rendered in its original color")]
fn image_color(world: &mut BehaviorWorld) {
    let case = world.case();
    let deadline = Instant::now() + Duration::from_secs(5);
    let color = ratatui::style::Color::Rgb(10, 20, 30);
    loop {
        crate::ui::image::pump();
        case.paint();
        if case.screen.contains_color(color) {
            return;
        }
        assert!(Instant::now() < deadline, "image color was not rendered");
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[then(expr = "the right quick view shows {string} as a {int} by {int} image")]
fn quick_view_image(world: &mut BehaviorWorld, name: String, width: u32, height: u32) {
    let case = world.case();
    let PanelKind::QuickView(view) = &case.app.right_panel.kind else {
        panic!("the right panel must be in quick view");
    };
    assert_eq!(
        view.shown.as_deref().and_then(|path| path.file_name()),
        Some(name.as_ref())
    );
    let screen = case.screen.text();
    assert!(screen.contains(&name), "missing {name:?}\n{screen}");
    let dimensions = format!("{width}×{height}");
    assert!(
        screen.contains(&dimensions),
        "missing {dimensions}\n{screen}"
    );
    assert!(screen.contains("PNG"), "missing image format\n{screen}");
}

#[then("the right panel is a file list")]
fn file_list(world: &mut BehaviorWorld) {
    assert!(matches!(
        world.case().app.right_panel.kind,
        PanelKind::Files
    ));
}
