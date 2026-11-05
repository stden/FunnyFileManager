use ratatui::Terminal;

use crate::models::{App, Duplicates, Overlay};
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
