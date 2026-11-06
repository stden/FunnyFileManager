//! Purpose: clicks in the Find duplicate blocks dialog: a text field takes the
//! cursor, Start and Cancel press their buttons.

use super::*;
use crate::models::{Duplicates, DuplicatesField};

fn open(dir: &TempDir) -> App {
    let mut app = app_at(dir);
    app.set_overlay(Overlay::Duplicates(Box::new(Duplicates::new("/work"))));
    app
}

fn field(app: &App) -> DuplicatesField {
    let Overlay::Duplicates(dialog) = &app.overlay else {
        panic!("the Find duplicate blocks dialog is open");
    };
    dialog.field()
}

#[test]
fn clicking_a_row_moves_the_cursor_to_its_text_field() {
    let dir = TempDir::new().expect("temp");
    let mut app = open(&dir);
    for (row, expected) in [
        (3, DuplicatesField::Mask),
        (5, DuplicatesField::MinBlock),
        (7, DuplicatesField::Groups),
        (1, DuplicatesField::Folder),
    ] {
        assert_eq!(click_dialog_row(&mut app, row), None);
        assert_eq!(field(&app), expected, "row {row}");
    }
}

#[test]
fn clicking_a_caption_or_below_the_fields_keeps_the_cursor() {
    let dir = TempDir::new().expect("temp");
    let mut app = open(&dir);
    click_dialog_row(&mut app, 5);
    for row in [0, 2, 4, 6, 8] {
        assert_eq!(click_dialog_row(&mut app, row), None);
        assert_eq!(field(&app), DuplicatesField::MinBlock, "row {row}");
    }
}

#[test]
fn the_buttons_press_enter_on_start_and_on_cancel() {
    let dir = TempDir::new().expect("temp");
    let mut app = open(&dir);
    assert_eq!(code(&mut app, 1), KeyCode::Enter);
    assert_eq!(field(&app), DuplicatesField::Cancel);
    assert_eq!(code(&mut app, 0), KeyCode::Enter);
    assert_eq!(field(&app), DuplicatesField::Start);
}

#[test]
fn clicking_the_drawn_cancel_button_closes_the_dialog_without_a_search() {
    let dir = TempDir::new().expect("temp");
    let mut app = open(&dir);
    let mut term = terminal();
    click_text(&mut term, &mut app, "Cancel");
    assert!(matches!(app.overlay, Overlay::None));
}
