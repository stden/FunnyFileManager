//! Purpose: Submitting the archive dialog's command without that dialog open is a
//! no-op: no archive is written and the overlay is left alone.

use super::super::submit;
use super::*;
use crate::models::ArchivePrompt;

#[test]
fn submitting_with_no_archive_dialog_open_writes_nothing() {
    let root = TempDir::new().expect("temporary directory");
    let mut app = App::with_panels(
        Panel::new(root.path().to_path_buf()).expect("panel"),
        Panel::new(root.path().to_path_buf()).expect("panel"),
    );

    submit(&mut app);

    assert!(matches!(app.overlay, Overlay::None));
    assert_eq!(fs::read_dir(root.path()).expect("listing").count(), 0);

    app.set_overlay(Overlay::ArchivePrompt(Box::new(ArchivePrompt::new(
        vec!["a.txt".into()],
        "a",
        root.path(),
        vec![ArchiveFormat::Zip],
    ))));
    process_popup_input(&mut app, key(KeyCode::Tab));
    process_popup_input(&mut app, key(KeyCode::Home));
    assert!(matches!(app.overlay, Overlay::ArchivePrompt(_)));
}
