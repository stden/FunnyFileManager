//! Unit tests: the Extract files dialog draws its caption, its destination
//! field, and Far's Extract / Cancel buttons — dimmed when there is nothing
//! to extract.

use std::path::{Path, PathBuf};

use ratatui::Terminal;

use crate::models::{App, ExtractField, ExtractPrompt, Overlay};
use crate::test_support::{buffer_text, WritableTestBackend};
use crate::ui::render_overlays;

fn draw(prompt: ExtractPrompt) -> (String, App) {
    let mut terminal = Terminal::new(WritableTestBackend::new(80, 24)).expect("terminal");
    let mut app = App::new().expect("test app");
    app.set_overlay(Overlay::ExtractPrompt(Box::new(prompt)));
    terminal
        .draw(|f| render_overlays(f, &mut app))
        .expect("draw");
    (buffer_text(&terminal), app)
}

#[test]
fn the_dialog_shows_the_caption_the_destination_and_the_buttons() {
    let prompt = ExtractPrompt::new(vec![PathBuf::from("/src/a.tar.gz")], Path::new("/dest"));
    let (screen, app) = draw(prompt);
    assert!(screen.contains("Extract files"), "{screen}");
    assert!(screen.contains("Extract \"a.tar.gz\" to:"), "{screen}");
    assert!(screen.contains("/dest"), "{screen}");
    assert!(screen.contains("{ Extract }"), "{screen}");
    assert!(screen.contains("[ Cancel ]"), "{screen}");
    assert!(!screen.contains("Coming in this release"), "{screen}");
    assert!(app.dialog_mouse.area.is_some(), "the frame was tracked");
    assert_eq!(app.dialog_mouse.buttons.len(), 2, "two buttons to click");
}

#[test]
fn the_focused_button_is_the_one_the_cursor_is_on() {
    let mut prompt = ExtractPrompt::new(vec![PathBuf::from("/src/a.tar")], Path::new("/dest"));
    prompt.field = ExtractField::Cancel;
    let (screen, _) = draw(prompt);
    // Far highlights the focused button; the text stays the same, so what the
    // frame proves is that the layout survives a cursor on Cancel.
    assert!(screen.contains("{ Extract }"), "{screen}");
    assert!(screen.contains("[ Cancel ]"), "{screen}");
}

#[test]
fn with_nothing_to_extract_the_caption_says_so() {
    let (screen, _) = draw(ExtractPrompt::new(Vec::new(), Path::new("/dest")));
    assert!(screen.contains("No archive selected"), "{screen}");
    assert!(screen.contains("{ Extract }"), "{screen}");
}
