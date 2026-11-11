use ratatui::{layout::Position, Terminal};

use crate::models::{App, Duplicates, DuplicatesField, Overlay};
use crate::test_support::{buffer_text, WritableTestBackend};

#[test]
fn the_dialog_shows_its_four_answers_and_both_buttons() {
    let mut terminal = Terminal::new(WritableTestBackend::new(90, 24)).expect("terminal");
    let mut app = App::new().expect("app");
    app.set_overlay(Overlay::Duplicates(Box::new(Duplicates::new(
        "/work/project",
    ))));
    terminal.draw(|f| crate::ui::ui(f, &mut app)).expect("draw");
    let screen = buffer_text(&terminal);
    for text in [
        "Find duplicate blocks",
        "Folder",
        "File mask",
        "Minimum block",
        "Groups to show",
        "/work/project",
        "Start",
        "Cancel",
    ] {
        assert!(screen.contains(text), "{text}:\n{screen}");
    }
}

/// The dialog drawn on its own, so the only caret is the one it places.
fn dialog_screen(field: DuplicatesField) -> (Terminal<WritableTestBackend>, String) {
    let mut terminal = Terminal::new(WritableTestBackend::new(90, 24)).expect("terminal");
    let mut dialog = Duplicates::new("/work/project");
    dialog.focus(field);
    let mut tracker = crate::mouse::DialogMouse::default();
    terminal
        .draw(|f| super::render(f, &mut tracker, &dialog))
        .expect("draw");
    let screen = buffer_text(&terminal);
    (terminal, screen)
}

#[test]
fn the_caret_sits_in_the_focused_text_field() {
    let (mut terminal, screen) = dialog_screen(DuplicatesField::Mask);
    let caret = terminal.get_cursor_position().expect("cursor");
    assert_ne!(caret, Position::new(0, 0), "{screen}");
    let row: String = (0..90)
        .map(|column| terminal.backend().buffer()[(column, caret.y)].symbol())
        .collect();
    assert!(row.contains('*'), "the caret is on the mask's row:\n{row}");
    assert!(!row.contains("project"), "not on the folder's row:\n{row}");
}

#[test]
fn a_focused_button_does_not_place_the_text_caret() {
    for field in [DuplicatesField::Start, DuplicatesField::Cancel] {
        let (mut terminal, screen) = dialog_screen(field);
        assert!(screen.contains("Find duplicate blocks"), "{screen}");
        assert_eq!(
            terminal.get_cursor_position().expect("cursor"),
            Position::new(0, 0),
            "{field:?}"
        );
    }
}
