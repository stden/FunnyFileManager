//! Purpose: Submitting the Apply dialog's command without that dialog open is a
//! no-op: no run, no transcript, no overlay change.

use super::super::submit;
use super::*;

#[test]
fn submitting_with_no_apply_dialog_open_does_nothing() {
    let root = TempDir::new().expect("temporary directory");
    let mut app = App::with_panels(
        Panel::new(root.path().to_path_buf()).expect("panel"),
        Panel::new(root.path().to_path_buf()).expect("panel"),
    );

    submit(&mut app);

    assert!(matches!(app.overlay, Overlay::None));
    assert!(app.console.is_empty());
}
