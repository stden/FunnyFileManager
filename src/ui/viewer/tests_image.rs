//! Status and errors for image viewers.

use super::*;
use crate::models::DecodedImage;
use std::sync::Arc;

#[test]
fn image_bar_reports_format_dimensions_size_and_zoom() {
    let mut viewer = Viewer::image("image.png".into());
    viewer.image_zoom = 2;
    viewer.image = Some(ImageState::Ready(DecodedImage {
        width: 2,
        height: 1,
        format: "PNG",
        size: 123,
        pixels: Arc::from([0u8; 8]),
    }));
    let row = bar_row(&viewer, Cells::columns(100), "12:34", 20);
    assert!(row.contains("PNG"));
    assert!(row.contains("2×1"));
    assert!(row.contains("123 bytes"));
    assert!(row.contains("2x"));
}

#[test]
fn loading_and_failed_images_have_clear_status() {
    let mut viewer = Viewer::image("image.png".into());
    assert!(bar_row(&viewer, Cells::columns(100), "12:34", 20).contains("Loading image"));
    viewer.image = Some(ImageState::Failed("bad image".into()));
    assert!(bar_row(&viewer, Cells::columns(100), "12:34", 20).contains("Image unavailable"));
}

#[test]
fn image_viewer_shows_a_compact_control_guide() {
    let mut app = tests::app_with(Viewer::image("image.png".into()));
    let terminal = tests::draw(&mut app, 80, 12);
    let footer = tests::row_text(&terminal, 11);
    for text in ["+/-", "Zoom", "0 Fit", "Arrows", "Esc"] {
        assert!(footer.contains(text), "missing {text} from {footer:?}");
    }
}

#[test]
fn narrow_image_guide_remains_useful_and_text_viewer_has_no_image_footer() {
    let mut image = tests::app_with(Viewer::image("image.png".into()));
    let terminal = tests::draw(&mut image, 12, 5);
    assert!(tests::row_text(&terminal, 4).contains("Esc"));

    let mut text = tests::app_with(Viewer::report("test", vec!["line".to_owned(); 4]));
    let terminal = tests::draw(&mut text, 80, 5);
    assert!(!tests::row_text(&terminal, 4).contains("Zoom"));
}

#[test]
fn loading_and_failed_images_render_a_clear_message() {
    let mut viewer = Viewer::image("image.png".into());
    viewer.image = Some(ImageState::Loading(7));
    let mut app = tests::app_with(viewer);
    let terminal = tests::draw(&mut app, 80, 5);
    assert!(tests::row_text(&terminal, 1).contains("Loading image"));

    let mut failed = Viewer::image("image.png".into());
    failed.image = Some(ImageState::Failed("bad header".to_owned()));
    let mut app = tests::app_with(failed);
    let terminal = tests::draw(&mut app, 80, 5);
    assert!(tests::row_text(&terminal, 1).contains("Cannot show image: bad header"));
    assert!(tests::row_text(&terminal, 2).contains("Enter to open externally"));
}

#[test]
fn one_body_row_remains_available_for_loading_and_middle_widths_show_guide() {
    let mut app = tests::app_with(Viewer::image("image.png".into()));
    let terminal = tests::draw(&mut app, 32, 2);
    assert!(tests::row_text(&terminal, 1).contains("Loading image"));

    let terminal = tests::draw(&mut app, 32, 5);
    let footer = tests::row_text(&terminal, 4);
    assert!(footer.contains("Arrows"));
    let terminal = tests::draw(&mut app, 20, 5);
    let footer = tests::row_text(&terminal, 4);
    assert!(footer.contains("0 Fit"));
    assert!(footer.contains("Esc"));
}

#[test]
fn ready_image_renders_preparation_and_preserves_the_control_row() {
    let mut viewer = Viewer::image("image.png".into());
    viewer.image = Some(ImageState::Ready(DecodedImage {
        width: 1,
        height: 1,
        format: "PNG",
        size: 10,
        pixels: Arc::from([255, 0, 0, 255]),
    }));
    let mut app = tests::app_with(viewer);
    let terminal = tests::draw(&mut app, 80, 5);
    assert!(tests::row_text(&terminal, 1).contains("Preparing image"));
    assert!(tests::row_text(&terminal, 4).contains("0 Fit"));
    crate::ui::image::clear();
}
