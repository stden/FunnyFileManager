//! Unit tests: the Apply command dialog draws Far's frame, its one field with
//! the text typed so far, the two buttons, and the caret where the editor's
//! cursor is.

use ratatui::Terminal;

use crate::models::{App, ApplyPrompt, Overlay};
use crate::test_support::{buffer_text, WritableTestBackend};
use crate::ui::render_overlays;

fn draw(prompt: ApplyPrompt) -> (Terminal<WritableTestBackend>, App) {
    let mut terminal = Terminal::new(WritableTestBackend::new(80, 24)).expect("test terminal");
    let mut app = App::new().expect("test app");
    app.set_overlay(Overlay::ApplyCommand(prompt));
    terminal
        .draw(|f| render_overlays(f, &mut app))
        .expect("draw");
    (terminal, app)
}

fn caret_x(prompt: ApplyPrompt) -> u16 {
    let (mut terminal, _) = draw(prompt);
    terminal.get_cursor_position().expect("cursor").x
}

#[test]
fn the_dialog_shows_its_title_field_and_buttons() {
    let mut prompt = ApplyPrompt::default();
    prompt.input_mut().insert_str("cp !.! !.bak");
    let (terminal, app) = draw(prompt);
    let screen = buffer_text(&terminal);
    assert!(screen.contains("Apply command"), "{screen}");
    assert!(screen.contains("Enter command:"), "{screen}");
    assert!(screen.contains("cp !.! !.bak"), "{screen}");
    assert!(
        screen.contains("OK") && screen.contains("Cancel"),
        "{screen}"
    );
    assert!(!screen.contains("Coming in this release"), "{screen}");
    assert!(app.dialog_mouse.area.is_some(), "the frame was tracked");
}

/// The caret follows the editor's cursor into the line rather than sitting
/// at its end, so what Left and Home did is visible.
#[test]
fn the_caret_sits_where_the_cursor_is() {
    let mut prompt = ApplyPrompt::default();
    prompt.input_mut().insert_str("cp !.! !.bak");
    let at_end = caret_x(prompt.clone());
    for _ in 0..4 {
        prompt.input_mut().left();
    }
    assert_eq!(caret_x(prompt), at_end - 4);
}

/// Far inserts names verbatim and the shell parses the result, so a file
/// called `$(touch x)` runs `touch x`. The dialog says so before Enter.
#[test]
fn the_dialog_warns_that_names_reach_the_shell_unescaped() {
    let (terminal, _) = draw(ApplyPrompt::default());
    let screen = buffer_text(&terminal);
    assert!(screen.contains("names are inserted unescaped"), "{screen}");
    assert!(screen.contains("can run commands"), "{screen}");
}
