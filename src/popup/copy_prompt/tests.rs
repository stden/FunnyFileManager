//! Purpose: Submitting the copy dialog without that dialog open is a no-op.

use super::*;
use crate::models::Panel;
use tempfile::TempDir;

#[test]
fn submitting_with_no_copy_dialog_open_does_nothing() {
    let root = TempDir::new().expect("temporary directory");
    let mut app = App::with_panels(
        Panel::new(root.path().to_path_buf()).expect("panel"),
        Panel::new(root.path().to_path_buf()).expect("panel"),
    );

    submit(&mut app);

    assert!(matches!(app.overlay, Overlay::None));
}
