//! Filter editing paths reached by keyboard and reused panel state.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tempfile::TempDir;

use crate::lineedit::LineEditor;
use crate::mask::Mask;
use crate::models::{App, FilterField, Overlay, Panel};
use crate::popup::process_popup_input;

fn fixture() -> (TempDir, App) {
    let temp = TempDir::new().expect("temp");
    for name in ["one.rs", "two.txt"] {
        std::fs::write(temp.path().join(name), "data").expect("file");
    }
    let panel = Panel::new(temp.path().to_owned()).expect("panel");
    let app = App::from_panels(panel.clone(), panel);
    (temp, app)
}

fn press(app: &mut App, code: KeyCode) {
    process_popup_input(app, code.into());
}

#[test]
fn changing_an_existing_panel_mask_keeps_its_source_and_refreshes_the_listing() {
    let (_temp, mut app) = fixture();
    app.left_panel.filter.include = Some(Mask::parse("*.txt"));
    app.left_panel.load_entries().expect("initial filter");
    crate::ops::filter::open(&mut app);
    process_popup_input(
        &mut app,
        KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL),
    );
    crate::popup::paste_text(&mut app, "*.RS");
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.left_panel.options.filter, "");
    assert_eq!(
        app.left_panel.filter.include.as_ref().map(Mask::source),
        Some("*.RS")
    );
    let names: Vec<_> = app
        .left_panel
        .entries
        .iter()
        .map(|entry| entry.name.as_str())
        .collect();
    assert!(names.contains(&"one.rs"));
    assert!(!names.contains(&"two.txt"));
}

#[test]
fn reverse_navigation_reaches_cancel_and_space_on_apply_commits() {
    let (_temp, mut app) = fixture();
    crate::ops::filter::open(&mut app);
    crate::popup::paste_text(&mut app, "*.rs");
    press(&mut app, KeyCode::Up);
    assert_eq!(app.filter_input.field, FilterField::Cancel);
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.left_panel.options.filter, "");
    crate::ops::filter::open(&mut app);
    press(&mut app, KeyCode::BackTab);
    assert_eq!(app.filter_input.field, FilterField::Cancel);
    press(&mut app, KeyCode::Char(' '));
    assert!(!app.overlay.is_active());
    crate::ops::filter::open(&mut app);
    crate::popup::paste_text(&mut app, "*.rs");
    for _ in 0..3 {
        press(&mut app, KeyCode::Tab);
    }
    assert_eq!(app.filter_input.field, FilterField::Apply);
    press(&mut app, KeyCode::Char(' '));
    assert_eq!(app.left_panel.options.filter, "*.rs");
    assert!(!app.overlay.is_active());
}

#[test]
fn extra_mask_accepts_alt_focus_and_unicode_paste_without_changing_the_first() {
    let (_temp, mut app) = fixture();
    app.left_panel.options.filter = "*.rs".to_owned();
    app.left_panel.filter.include = Some(Mask::parse("*one*"));
    crate::ops::filter::open(&mut app);
    process_popup_input(
        &mut app,
        KeyEvent::new(KeyCode::Char('x'), KeyModifiers::ALT | KeyModifiers::SHIFT),
    );
    assert_eq!(app.filter_input.field, FilterField::Additional);
    crate::popup::paste_text(&mut app, "файл");
    assert_eq!(
        app.filter_input.additional.as_ref().map(LineEditor::text),
        Some("*one*файл")
    );
    let Overlay::Filter(dialog) = &app.overlay else {
        panic!("filter")
    };
    assert_eq!(dialog.include, "*.rs");
    process_popup_input(
        &mut app,
        KeyEvent::new(KeyCode::Char('z'), KeyModifiers::ALT),
    );
    assert_eq!(
        app.filter_input.additional.as_ref().map(LineEditor::text),
        Some("*one*файл")
    );
}

#[test]
fn keys_and_apply_leave_an_unrelated_overlay_untouched() {
    let (_temp, mut app) = fixture();
    app.set_overlay(Overlay::Help);
    assert!(!super::process(
        &mut app,
        KeyCode::Char('a'),
        KeyModifiers::NONE
    ));
    crate::ops::filter::apply(&mut app);
    assert!(matches!(app.overlay, Overlay::Help));
}

#[test]
fn typing_on_a_checkbox_does_not_change_the_masks_or_the_checkbox() {
    let (_temp, mut app) = fixture();
    crate::ops::filter::open(&mut app);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Tab);
    assert_eq!(app.filter_input.field, FilterField::Hidden);
    press(&mut app, KeyCode::Char('x'));
    let Overlay::Filter(dialog) = &app.overlay else {
        panic!("filter")
    };
    assert_eq!(dialog.include, "");
    assert_eq!(dialog.exclude, "");
    assert!(!dialog.hide_dotfiles);
}
