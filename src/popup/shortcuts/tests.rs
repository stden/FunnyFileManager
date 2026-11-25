use crossterm::event::KeyCode;

use crate::models::{App, Overlay};
use crate::popup::process_popup_input;
use crate::popup::tests::key;

/// Opening sets the variant; Esc takes it down and runs nothing else.
#[test]
fn open_sets_the_variant_and_esc_closes_it() {
    let mut app = App::new().expect("test setup");
    crate::ops::shortcuts::open(&mut app);
    assert!(matches!(app.overlay, Overlay::FolderShortcuts(_)));

    process_popup_input(&mut app, key(KeyCode::Char('x')));
    assert!(
        matches!(app.overlay, Overlay::FolderShortcuts(_)),
        "other keys are consumed"
    );

    process_popup_input(&mut app, key(KeyCode::Esc));
    assert!(!app.overlay.is_active());
}
