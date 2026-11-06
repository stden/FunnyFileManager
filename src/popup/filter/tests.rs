use crossterm::event::{KeyCode, KeyModifiers};

use crate::models::{App, Overlay};
use crate::popup::process_popup_input;
use crate::popup::tests::key;

/// Opening sets the variant; Esc takes it down and runs nothing else.
#[test]
fn open_sets_the_variant_and_esc_closes_it() {
    let mut app = App::new().expect("test setup");
    crate::ops::filter::open(&mut app);
    assert!(matches!(app.overlay, Overlay::Filter(_)));

    process_popup_input(&mut app, key(KeyCode::Char('x')));
    assert!(
        matches!(app.overlay, Overlay::Filter(_)),
        "other keys are consumed"
    );

    process_popup_input(&mut app, key(KeyCode::Esc));
    assert!(!app.overlay.is_active());
}

#[test]
fn a_missing_optional_filter_field_does_not_edit_another_field() {
    let mut app = App::new().expect("app");
    app.filter_input.field = crate::models::FilterField::Additional;
    app.filter_input.additional = None;
    let before = app.filter_input.include.text().to_owned();
    super::edit(&mut app, KeyCode::Char('x'), KeyModifiers::NONE);
    assert_eq!(app.filter_input.include.text(), before);
    assert!(app.filter_input.additional.is_none());
}
