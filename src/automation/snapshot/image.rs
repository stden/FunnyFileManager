//! Purpose: Publish image decoder state and bounded metadata without serializing RGBA pixels.

use crate::models::{ImageState, Overlay};
use serde_json::{json, Value};

pub(super) fn viewer(overlay: &Overlay) -> Value {
    let Overlay::Viewer(viewer) = overlay else {
        return Value::Null;
    };
    let mut value = viewer.image.as_ref().map_or(Value::Null, describe);
    if let Some(fields) = value.as_object_mut() {
        drop(fields.insert("zoom".to_owned(), json!(viewer.image_zoom)));
        drop(fields.insert(
            "pan".to_owned(),
            json!({"x":viewer.image_pan.0,"y":viewer.image_pan.1}),
        ));
    }
    value
}

pub(super) fn describe(state: &ImageState) -> Value {
    match state {
        ImageState::Pending => json!({"kind":"image","state":"pending"}),
        ImageState::Loading(_) => json!({"kind":"image","state":"loading"}),
        ImageState::Ready(image) => json!({"kind":"image","state":"ready","width":image.width(),
            "height":image.height(),"format":image.format(),"size":image.size()}),
        ImageState::Failed(reason) => json!({"kind":"image","state":"failed","reason":reason}),
    }
}

pub(super) const fn is_pending(state: &ImageState) -> bool {
    matches!(state, ImageState::Pending | ImageState::Loading(_))
}

#[cfg(test)]
mod tests;
