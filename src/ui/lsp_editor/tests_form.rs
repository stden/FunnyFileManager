//! Purpose: Pin the form on screen: its fields, the dimmed language of an
//! existing server, the caret in a scrolled field, and the dropdown.

use super::*;
use crate::models::lsp_editor::{FormField, Mode};
use crate::models::ExitChoice;

fn form_on(app: &mut App, language: &str) {
    select(app, language);
    press(app, KeyCode::F(4));
}

fn draw_with_cursor(app: &mut App, width: u16, height: u16) -> (Buffer, (u16, u16)) {
    let mut terminal = Terminal::new(WritableTestBackend::new(width, height)).expect("terminal");
    terminal.draw(|f| crate::ui::ui(f, app)).expect("draw");
    let cursor = terminal.get_cursor_position().expect("cursor");
    (terminal.backend().buffer().clone(), (cursor.x, cursor.y))
}

#[test]
fn an_existing_servers_form_dims_its_language() {
    let machine = LspMachine::new();
    let mut app = machine.opened();
    form_on(&mut app, "python");
    let buffer = draw(&mut app, 80, 24);
    let screen = buffer_string(&buffer);
    assert!(screen.contains(" Edit server: python "), "{screen}");
    assert!(screen.contains("{ OK }"), "{screen}");
    assert!(screen.contains("[ Cancel ]"), "{screen}");
    assert!(screen.contains("Program name on PATH, or a full path. No shell runs it."));
    let (x, y) = find(&buffer, "Language:");
    assert_eq!(buffer[(x, y)].fg, palette::DISABLED_FG);
    let (x, y) = find(&buffer, "Command:");
    assert_ne!(buffer[(x, y)].fg, palette::DISABLED_FG);
}

#[test]
fn a_new_servers_form_is_titled_add_server() {
    let machine = LspMachine::new();
    let mut app = machine.opened();
    press(&mut app, KeyCode::Insert);
    let screen = buffer_string(&draw(&mut app, 80, 24));
    assert!(screen.contains(" Add server "), "{screen}");
    assert!(
        screen.contains("The LSP languageId: a-z 0-9 - _ . +"),
        "{screen}"
    );
}

/// A long value scrolls to keep the caret in view, measured in cells, so a
/// wide character does not push the caret off its place.
#[test]
fn the_caret_stays_in_a_scrolled_field_with_wide_characters() {
    let machine = LspMachine::new();
    let mut app = machine.opened();
    form_on(&mut app, "python");
    let long = format!("/srv/x/{}/pylsp", "語".repeat(40));
    let form = editor_mut(&mut app).form_mut().expect("form");
    form.focused_line_mut().expect("command").set_text(&long);
    let (buffer, (x, y)) = draw_with_cursor(&mut app, 80, 24);
    let (field_x, field_y) = find(&buffer, "Command:");
    assert_eq!(y, field_y);
    let row: String = (field_x..x)
        .map(|column| buffer[(column, y)].symbol())
        .collect();
    assert!(row.ends_with("pylsp"), "{row:?}");
    assert!(row.contains('…'), "the head is cut: {row:?}");
}

/// The python form with `vscode` typed in its command, which opens a
/// passive list; the screen, and the command field's row.
fn typed_vscode(app: &mut App) -> (Buffer, u16) {
    form_on(app, "python");
    for _ in 0..5 {
        press(app, KeyCode::Backspace);
    }
    for ch in "vscode".chars() {
        press(app, KeyCode::Char(ch));
    }
    let buffer = draw(app, 80, 24);
    let (_, field_y) = find(&buffer, "Command:");
    (buffer, field_y)
}

#[test]
fn the_dropdown_hangs_below_its_field() {
    let machine = LspMachine::new();
    let mut app = machine.opened();
    let (buffer, field_y) = typed_vscode(&mut app);
    let (_, y) = find_below(&buffer, "vscode-css-language-server", field_y).expect("the list");
    assert_eq!(y, field_y + 2, "under the dropdown's top frame");
    let (field_x, _) = find(&buffer, "Command:");
    let frame = &buffer[(field_x + 12, field_y + 1)];
    assert_eq!(frame.symbol(), "╔");
    assert_eq!(frame.fg, palette::COMPLETION_BG);
}

/// A passive list highlights no row; Down makes it active on its first.
#[test]
fn a_passive_list_highlights_nothing_until_down() {
    let machine = LspMachine::new();
    let mut app = machine.opened();
    let (buffer, field_y) = typed_vscode(&mut app);
    let (x, y) = find_below(&buffer, "vscode-css-language-server", field_y).expect("the list");
    assert_ne!(buffer[(x, y)].bg, palette::COMPLETION_BG, "passive");
    press(&mut app, KeyCode::Down);
    let buffer = draw(&mut app, 80, 24);
    let (x, y) = find_below(&buffer, "vscode-css-language-server", field_y).expect("the list");
    assert_eq!(buffer[(x, y)].bg, palette::COMPLETION_BG, "active");
}

/// A form dragged near the bottom of the screen hangs its list above the
/// field instead.
#[test]
fn the_dropdown_flips_above_when_there_is_no_room_below() {
    let machine = LspMachine::new();
    let mut app = machine.opened();
    form_on(&mut app, "python");
    for _ in 0..2 {
        press(&mut app, KeyCode::Tab);
    }
    assert_eq!(
        editor_mut(&mut app).form_mut().expect("form").focus(),
        FormField::Extensions
    );
    for ch in " j".chars() {
        press(&mut app, KeyCode::Char(ch));
    }
    press_with(&mut app, KeyCode::Char(' '), KeyModifiers::CONTROL);
    app.dialog_mouse.offset = (0, 20);
    let buffer = draw(&mut app, 80, 24);
    let (field_x, field_y) = find(&buffer, "Extensions:");
    let frame = &buffer[(field_x + 12, field_y - 1)];
    assert_eq!(frame.symbol(), "╚", "{}", buffer_string(&buffer));
    assert_eq!(frame.fg, palette::COMPLETION_BG);
    assert!(
        find_below(&buffer, ".json", field_y).is_none(),
        "nothing below"
    );
}

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
