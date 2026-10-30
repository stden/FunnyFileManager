//! Unit tests: a dialog's controls — checkbox styles, a label ending in the
//! hotkey marker, and Far's control and button colours on the prompt and
//! warning grounds.

use ratatui::backend::TestBackend;
use ratatui::Terminal;

use super::overlay_mask::screen_text;
use super::*;

// -----------------------------------------------------------------------
// dialog::render_at row styles, exercised directly against the dialog
// primitives rather than through any one overlay that happens to use them.
// -----------------------------------------------------------------------

/// A disabled checkbox is dimmed even off the cursor, and a focused enabled
/// one is highlighted — two different reasons for two different styles on the
/// very same row type, so mixing them up would draw the wrong one dim or the
/// wrong one lit.
#[test]
fn checkbox_rows_pick_the_dimmed_or_the_focused_style() {
    use super::dialog::{self, Button, Ground, Row};

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let rows = [
        Row::Checkbox {
            label: "&Disabled here",
            checked: false,
            focused: false,
            enabled: false,
        },
        Row::Checkbox {
            label: "&Focused here",
            checked: true,
            focused: true,
            enabled: true,
        },
    ];
    let buttons = [Button::new("OK").default_button()];

    terminal
        .draw(|f| {
            dialog::render_at(f, "Checks", Ground::Prompt, &rows, &buttons, (0, 0));
        })
        .unwrap();

    let screen = screen_text(&terminal);
    assert!(screen.contains("Disabled here"), "{screen}");
    assert!(screen.contains("Focused here"), "{screen}");
}

/// A row label that ends right on the hotkey marker has no letter left to
/// accent — the text before the marker must still be drawn, not panic on the
/// missing character.
#[test]
fn a_row_label_ending_in_the_hotkey_marker_keeps_its_text() {
    use super::dialog::{self, Button, Ground, Row};

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let rows = [Row::Text("Trailing marker&")];
    let buttons = [Button::new("OK").default_button()];

    terminal
        .draw(|f| {
            dialog::render_at(f, "Edge", Ground::Prompt, &rows, &buttons, (0, 0));
        })
        .unwrap();

    assert!(screen_text(&terminal).contains("Trailing marker"));
}

fn draw_dialog_controls(ground: dialog::Ground) -> Terminal<TestBackend> {
    use super::dialog::{self, Button, Row};
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal
        .draw(|frame| {
            dialog::render_at(
                frame,
                "Colors",
                ground,
                &[Row::Label("&Name"), Row::input("value", true, false)],
                &[
                    Button::new("&OK").focused(true),
                    Button::new("&Apply").enabled(false),
                ],
                (0, 0),
            );
        })
        .unwrap();
    terminal
}

fn assert_dialog_control_colors(ground: dialog::Ground, button_bg: Color) {
    let terminal = draw_dialog_controls(ground);
    let buffer = terminal.backend().buffer();
    let field = buffer
        .content
        .iter()
        .find(|cell| cell.symbol() == "v")
        .unwrap();
    assert_eq!(
        (field.fg, field.bg),
        (Color::Rgb(0, 0, 0), Color::Rgb(0, 128, 128))
    );
    let button = buffer
        .content
        .iter()
        .find(|cell| cell.symbol() == "K")
        .unwrap();
    assert_eq!((button.fg, button.bg), (Color::Rgb(0, 0, 0), button_bg));
    let hotkey = buffer
        .content
        .iter()
        .find(|cell| cell.symbol() == "N")
        .unwrap();
    assert_eq!(hotkey.fg, Color::Rgb(255, 255, 0));
    let background = match ground {
        dialog::Ground::Prompt => Color::Rgb(192, 192, 192),
        dialog::Ground::Warning => Color::Rgb(128, 0, 0),
    };
    assert_eq!(hotkey.bg, background);
    assert_button_accents(buffer, button_bg, background);
}

fn assert_button_accents(buffer: &ratatui::buffer::Buffer, focused: Color, ground: Color) {
    let hotkey = buffer
        .content
        .iter()
        .find(|cell| cell.symbol() == "O")
        .unwrap();
    assert_eq!((hotkey.fg, hotkey.bg), (Color::Rgb(255, 255, 0), focused));
    let disabled = buffer
        .content
        .iter()
        .find(|cell| cell.symbol() == "A")
        .unwrap();
    assert_eq!(
        (disabled.fg, disabled.bg),
        (Color::Rgb(128, 128, 128), ground)
    );
}

#[test]
fn prompt_controls_use_far_black_on_cyan() {
    assert_dialog_control_colors(dialog::Ground::Prompt, Color::Rgb(0, 128, 128));
}

#[test]
fn warning_controls_use_far_gray_selected_button() {
    assert_dialog_control_colors(dialog::Ground::Warning, Color::Rgb(192, 192, 192));
}
