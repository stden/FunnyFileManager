//! Purpose: Quick-view image snapshots expose decoder metadata rather than hidden local entries.

use super::*;
use crate::models::{ImageState, Preview, QuickView};

#[test]
fn quick_view_image_has_explicit_decoder_state_without_file_list_rows() {
    let root = TempDir::new().unwrap();
    let mut panel = hidden_panel(&root);
    panel.kind = PanelKind::QuickView(Box::new(QuickView {
        shown: Some(root.path().join("fixture.png")),
        preview: Preview::Image(ImageState::Loading(21)),
        ..QuickView::default()
    }));
    let value = describe(&panel);
    assert_eq!(value["preview"]["kind"], "image");
    assert_eq!(value["preview"]["state"], "loading");
    assert_eq!(value["entries"], json!([]));
    assert!(!value.to_string().contains("hidden-local-entry"));
}
