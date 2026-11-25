//! Purpose: Pin the form's controls and the exit question on screen: each
//! body row draws the control a click reaches, a long value is cut inside its
//! box, only the focused control stands out, and the chosen button is marked.

use super::form::form_on;
use super::*;
use crate::models::lsp_editor::{FormField, Mode};
use crate::models::ExitChoice;

#[test]
fn the_exit_question_is_the_editors() {
    let machine = LspMachine::new();
    let mut app = machine.opened();
    press(&mut app, KeyCode::Char(' '));
    press(&mut app, KeyCode::Esc);
    let buffer = draw(&mut app, 80, 24);
    let screen = buffer_string(&buffer);
    assert!(screen.contains("Language server settings have been modified. Save?"));
    assert!(screen.contains("{ Yes }"), "{screen}");
    assert_eq!(buffer[find(&buffer, "Save?")].bg, palette::WARNING_BG);
}

/// The caption `ui` draws for each body control.
fn caption(field: FormField) -> &'static str {
    match field {
        FormField::Language => "Language:",
        FormField::Command => "Command:",
        FormField::Arguments => "Arguments:",
        FormField::Extensions => "Extensions:",
        FormField::Enabled => "Enabled",
        FormField::Ok | FormField::Cancel => panic!("buttons are not body rows"),
    }
}

/// `FormField::at_row` is what a click reaches, so each body row must draw
/// the control it names, and the rows after the controls none.
#[test]
fn each_body_row_draws_the_control_a_click_on_it_reaches() {
    let machine = LspMachine::new();
    let mut app = machine.opened();
    press(&mut app, KeyCode::Insert);
    let buffer = draw(&mut app, 80, 24);
    let body = app.dialog_mouse.body.expect("the form is drawn");
    for index in 0..7 {
        let row: String = (body.x..body.x + body.width)
            .map(|x| buffer[(x, body.y + index)].symbol())
            .collect();
        match FormField::at_row(usize::from(index)) {
            Some(field) => assert!(row.contains(caption(field)), "{index}: {row}"),
            None => assert!(
                FormField::ROWS
                    .iter()
                    .all(|field| !row.contains(caption(*field))),
                "{index}: {row}"
            ),
        }
    }
}

/// The text of row `y` from column `x`, `count` cells long.
fn cells_from(buffer: &Buffer, (x, y): (u16, u16), count: u16) -> String {
    (x..x + count)
        .map(|column| buffer[(column, y)].symbol())
        .collect()
}

/// A field that does not have the cursor shows the head of a long value,
/// cut with `…` at the last cell its box holds, before the list arrow.
#[test]
fn a_long_value_is_cut_at_the_end_of_its_box() {
    let machine = LspMachine::new();
    let mut app = machine.opened();
    form_on(&mut app, "python");
    let form = editor_mut(&mut app).form_mut().expect("form");
    form.focus_on(FormField::Arguments);
    form.focused_line_mut()
        .expect("arguments")
        .set_text(&"--x ".repeat(30));
    form.focus_on(FormField::Command);
    let buffer = draw(&mut app, 80, 24);
    let expected = "Arguments:  --x --x --x --x --x --x --x --x --x --x --x…↓║";
    let count = u16::try_from(expected.chars().count()).expect("short");
    assert_eq!(
        cells_from(&buffer, find(&buffer, "Arguments:"), count),
        expected
    );
}

/// The backgrounds of `needles` on screen, in order.
fn grounds(buffer: &Buffer, needles: &[&str]) -> Vec<ratatui::style::Color> {
    needles
        .iter()
        .map(|needle| buffer[find(buffer, needle)].bg)
        .collect()
}

/// Of `needles`, only the one at `focused` has its own ground.
fn assert_only_focused_stands_out(buffer: &Buffer, needles: &[&str], focused: usize) {
    let grounds = grounds(buffer, needles);
    for (index, ground) in grounds.iter().enumerate() {
        if index == focused {
            continue;
        }
        assert_ne!(*ground, grounds[focused], "{needles:?} focus on {focused}");
        assert_eq!(
            *ground,
            grounds[(focused + 1) % needles.len()],
            "{needles:?}"
        );
    }
}

/// The control with the cursor is drawn as focused and no other: a bold
/// value for a field, the focus ground for the checkbox and each button.
#[test]
fn only_the_focused_control_is_drawn_as_focused() {
    let machine = LspMachine::new();
    let mut app = machine.opened();
    form_on(&mut app, "python");
    let form = editor_mut(&mut app).form_mut().expect("form");
    form.focus_on(FormField::Arguments);
    form.focused_line_mut()
        .expect("arguments")
        .set_text("--log");
    form.focus_on(FormField::Command);
    let buffer = draw(&mut app, 80, 24);
    assert!(buffer[find(&buffer, "pylsp")]
        .modifier
        .contains(Modifier::BOLD));
    assert!(!buffer[find(&buffer, "--log")]
        .modifier
        .contains(Modifier::BOLD));
    assert_eq!(cells_from(&buffer, find(&buffer, "[x] Enabled"), 3), "[x]");

    let controls = ["[x] Enabled", "{ OK }", "[ Cancel ]"];
    for (index, focus) in [FormField::Enabled, FormField::Ok, FormField::Cancel]
        .into_iter()
        .enumerate()
    {
        editor_mut(&mut app)
            .form_mut()
            .expect("form")
            .focus_on(focus);
        assert_only_focused_stands_out(&draw(&mut app, 80, 24), &controls, index);
    }
}

/// The exit question's focus follows its choice.
#[test]
fn the_exit_question_marks_the_chosen_button() {
    let machine = LspMachine::new();
    let mut app = machine.opened();
    press(&mut app, KeyCode::Char(' '));
    press(&mut app, KeyCode::Esc);
    let buttons = ["{ Yes }", "[ No ]", "[ Cancel ]"];
    for (index, choice) in [ExitChoice::Save, ExitChoice::Discard, ExitChoice::Cancel]
        .into_iter()
        .enumerate()
    {
        editor_mut(&mut app).set_mode(Mode::ConfirmExit(choice));
        assert_only_focused_stands_out(&draw(&mut app, 80, 24), &buttons, index);
    }
}
