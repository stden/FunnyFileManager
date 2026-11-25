//! Purpose: Image snapshots expose readiness and dimensions while excluding pixel storage.

use super::*;
use crate::models::{DecodedImage, Viewer};
use std::sync::Arc;

fn ready() -> ImageState {
    ImageState::Ready(DecodedImage {
        width: 2,
        height: 1,
        format: "PNG",
        size: 57,
        pixels: Arc::from([255, 0, 0, 255, 0, 255, 0, 255]),
    })
}

#[test]
fn decoded_metadata_has_no_pixel_payload() {
    let value = describe(&ready());
    assert_eq!(value["state"], "ready");
    assert_eq!(value["width"], 2);
    assert_eq!(value["height"], 1);
    assert_eq!(value["format"], "PNG");
    assert_eq!(value["size"], 57);
    assert!(value.get("pixels").is_none());
    assert!(!is_pending(&ready()));
}

#[test]
fn pending_loading_and_failed_states_are_explicit() {
    for (state, expected, pending) in [
        (ImageState::Pending, "pending", true),
        (ImageState::Loading(12), "loading", true),
        (ImageState::Failed("fixture".into()), "failed", false),
    ] {
        let value = describe(&state);
        assert_eq!(value["state"], expected);
        assert_eq!(is_pending(&state), pending);
    }
}

#[test]
fn full_screen_image_metadata_is_absent_for_ordinary_text_viewers() {
    assert!(viewer(&Overlay::Help).is_null());
    let mut opened = Viewer::report("fixture", vec!["text".into()]);
    assert!(viewer(&Overlay::Viewer(opened.clone())).is_null());
    opened.image = Some(ready());
    opened.image_zoom = 2;
    opened.image_pan = (1, 0);
    let value = viewer(&Overlay::Viewer(opened));
    assert_eq!(value["width"], 2);
    assert_eq!(value["zoom"], 2);
    assert_eq!(value["pan"]["x"], 1);
}
