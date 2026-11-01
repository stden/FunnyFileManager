//! Image zoom and panning keep the normal viewer close key.

use super::*;

fn ready_image() -> Viewer {
    let mut viewer = Viewer::image("sample.png".into());
    viewer.image = Some(crate::models::ImageState::Ready(
        crate::models::DecodedImage {
            width: 256,
            height: 128,
            format: "PNG",
            size: 50,
            pixels: std::sync::Arc::from(vec![0; 256 * 128 * 4]),
        },
    ));
    viewer
}

#[test]
fn image_controls_zoom_pan_reset_and_close() {
    let mut app = App::new().expect("app");
    app.set_overlay(Overlay::Viewer(ready_image()));
    assert!(process(&mut app, KeyCode::Char('+'), KeyModifiers::SHIFT));
    assert!(process(&mut app, KeyCode::Right, KeyModifiers::NONE));
    let viewer = app.get_viewer().expect("viewer");
    assert_eq!(viewer.image_zoom, 2);
    assert_eq!(viewer.image_pan, (64, 0));
    assert!(process(&mut app, KeyCode::Char('0'), KeyModifiers::NONE));
    let viewer = app.get_viewer().expect("viewer");
    assert_eq!(viewer.image_zoom, 1);
    assert_eq!(viewer.image_pan, (0, 0));
    assert!(process(&mut app, KeyCode::Esc, KeyModifiers::NONE));
    assert!(app.get_viewer().is_none());
}

#[test]
fn panning_stops_at_edge_and_reverses_immediately() {
    let mut viewer = ready_image();
    assert!(image_key(&mut viewer, KeyCode::Char('+')));
    for _ in 0..20 {
        assert!(image_key(&mut viewer, KeyCode::Right));
    }
    assert_eq!(viewer.image_pan.0, 128);
    assert!(image_key(&mut viewer, KeyCode::Left));
    assert_eq!(viewer.image_pan.0, 64);
}

#[test]
fn panning_fit_and_zooming_out_keep_coordinates_in_bounds() {
    let mut viewer = ready_image();
    for _ in 0..5 {
        assert!(image_key(&mut viewer, KeyCode::Right));
    }
    assert_eq!(viewer.image_pan, (0, 0));
    assert!(image_key(&mut viewer, KeyCode::Char('+')));
    assert!(image_key(&mut viewer, KeyCode::Char('+')));
    for _ in 0..5 {
        assert!(image_key(&mut viewer, KeyCode::Right));
    }
    assert_eq!(viewer.image_pan.0, 171);
    assert!(image_key(&mut viewer, KeyCode::Char('-')));
    assert_eq!(viewer.image_pan.0, 128);
    assert!(image_key(&mut viewer, KeyCode::Char('-')));
    assert_eq!(viewer.image_pan, (0, 0));
}

#[test]
fn vertical_panning_stops_at_the_edge_and_reverses() {
    let mut viewer = ready_image();
    assert!(image_key(&mut viewer, KeyCode::Char('+')));
    for _ in 0..5 {
        assert!(image_key(&mut viewer, KeyCode::Down));
    }
    assert_eq!(viewer.image_pan.1, 64);
    assert!(image_key(&mut viewer, KeyCode::Up));
    assert_eq!(viewer.image_pan.1, 0);
}

#[test]
fn arrow_keys_while_loading_do_not_accumulate_hidden_pan() {
    let mut viewer = Viewer::image("sample.png".into());
    assert!(image_key(&mut viewer, KeyCode::Char('+')));
    assert!(image_key(&mut viewer, KeyCode::Down));
    assert_eq!(viewer.image_pan, (0, 0));
    assert!(image_key(&mut viewer, KeyCode::Right));
    assert_eq!(viewer.image_pan, (0, 0));
}
