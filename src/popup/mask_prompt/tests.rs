//! Purpose: the Select / Deselect field offers a mask the way Far's does —
//! pre-selected, so the first key typed replaces it instead of appending.

use std::fs;

use crossterm::event::KeyCode;
use tempfile::TempDir;

use crate::models::{App, MaskPrompt, MaskPurpose, Overlay, Panel};
use crate::popup::process_popup_input;
use crate::test_support::key_event;

fn app_over(names: &[&str]) -> (App, TempDir) {
    let dir = TempDir::new().expect("test setup");
    for name in names {
        fs::write(dir.path().join(name), b"x").expect("test setup");
    }
    let path = dir.path().to_path_buf();
    let left = Panel::new(path.clone()).expect("test setup");
    let right = Panel::new(path).expect("test setup");
    (App::with_panels(left, right), dir)
}

fn open_offering(app: &mut App, offered: &str) {
    app.set_overlay(Overlay::MaskPrompt(MaskPrompt::new(
        MaskPurpose::Select,
        offered,
    )));
}

fn press(app: &mut App, code: KeyCode) {
    process_popup_input(app, key_event(code));
}

fn type_text(app: &mut App, text: &str) {
    text.chars().for_each(|ch| press(app, KeyCode::Char(ch)));
}

fn prompt(app: &App) -> &MaskPrompt {
    let Overlay::MaskPrompt(prompt) = &app.overlay else {
        panic!("the mask dialog is open");
    };
    prompt
}

fn field(app: &App) -> &str {
    prompt(app).mask()
}

fn marked(app: &mut App) -> Vec<String> {
    let panel = app.get_active_panel();
    panel
        .marked
        .iter()
        .map(|name| name.as_str().to_owned())
        .collect()
}

/// The reported case: `*.txt` offered, `semi*` typed, and the field used to
/// read `*.txtsemi*` — a mask that matches nothing.
#[test]
fn typing_replaces_the_offered_mask() {
    let (mut app, _dir) = app_over(&["semi;colon", "notes.txt"]);
    open_offering(&mut app, "*.txt");
    assert!(prompt(&app).is_offered());

    type_text(&mut app, "semi*");
    assert_eq!(field(&app), "semi*");
    assert!(
        !prompt(&app).is_offered(),
        "the typed text is the user's own"
    );

    press(&mut app, KeyCode::Enter);
    assert_eq!(marked(&mut app), ["semi;colon"]);
    assert_eq!(app.status_text(), "1 changed, 1 selected");
}

/// Only the first key replaces; once the caret has moved, the field is the
/// user's own text and typing edits it where the caret stands.
#[test]
fn once_edited_the_field_is_no_longer_replaced() {
    let (mut app, _dir) = app_over(&[]);
    open_offering(&mut app, "*.txt");
    press(&mut app, KeyCode::End);
    press(&mut app, KeyCode::Backspace);
    type_text(&mut app, "t");
    assert_eq!(
        field(&app),
        "*.txt",
        "End keeps the offer, then it is edited"
    );
}

/// The offer is selected text: Backspace removes it all, as in any field.
#[test]
fn backspace_on_the_offer_clears_it() {
    let (mut app, _dir) = app_over(&[]);
    open_offering(&mut app, "*.txt");
    press(&mut app, KeyCode::Backspace);
    assert_eq!(field(&app), "");
}

/// Accepting the offer untouched still applies it.
#[test]
fn enter_applies_the_offered_mask_as_it_stands() {
    let (mut app, _dir) = app_over(&["notes.txt", "main.rs"]);
    open_offering(&mut app, "*.txt");
    press(&mut app, KeyCode::Enter);
    assert_eq!(marked(&mut app), ["notes.txt"]);
}
