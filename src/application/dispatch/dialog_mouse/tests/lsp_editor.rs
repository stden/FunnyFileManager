//! Purpose: Pin what the mouse does in the Language servers dialog: its form
//! and exit-question buttons are clicks, the wheel is Up and Down.

use super::*;
use crate::models::lsp_editor::{FormField, FormTarget, Mode};
use crate::test_support::lsp_machine::{editor, editor_mut, LspMachine};

/// A click on `{ OK }` or `[ Cancel ]` names the button before Enter, so an
/// old keyboard focus on Cancel cannot turn a click on OK into Cancel.
#[test]
fn a_form_button_click_focuses_it_before_enter() {
    let machine = LspMachine::new();
    let mut app = machine.opened();
    crate::ops::lsp_editor::open_form(&mut app, &FormTarget::New);
    editor_mut(&mut app)
        .form_mut()
        .expect("form")
        .focus_on(FormField::Cancel);
    assert_eq!(code(&mut app, 0), KeyCode::Enter);
    assert_eq!(
        editor_mut(&mut app).form_mut().expect("form").focus(),
        FormField::Ok
    );
    assert_eq!(code(&mut app, 1), KeyCode::Enter);
    assert_eq!(
        editor_mut(&mut app).form_mut().expect("form").focus(),
        FormField::Cancel
    );
}

#[test]
fn the_exit_questions_buttons_are_its_hotkeys() {
    let machine = LspMachine::new();
    let mut app = machine.opened();
    editor_mut(&mut app).set_mode(Mode::ConfirmExit(ExitChoice::Cancel));
    assert_eq!(code(&mut app, 0), KeyCode::Char('y'));
    assert_eq!(code(&mut app, 1), KeyCode::Char('n'));
    assert_eq!(code(&mut app, 2), KeyCode::Esc);
}

/// The list and the view have no buttons: a click there is nothing.
#[test]
fn the_list_and_the_view_have_no_buttons() {
    let machine = LspMachine::new();
    let mut app = machine.opened();
    assert!(dialog_button_key(&mut app, 0).is_none());
    editor_mut(&mut app).set_mode(Mode::View(crate::scroll::ScrollTop::TOP));
    assert!(dialog_button_key(&mut app, 0).is_none());
}

#[test]
fn the_wheel_moves_the_server_selection() {
    let machine = LspMachine::new();
    let mut app = machine.opened();
    let mut term = terminal();
    gesture(&mut term, &mut app, MouseEventKind::ScrollDown, (40, 12));
    assert_eq!(
        editor(&app)
            .selected()
            .map(crate::config::LanguageId::as_str),
        Some("css")
    );
    gesture(&mut term, &mut app, MouseEventKind::ScrollUp, (40, 12));
    assert_eq!(
        editor(&app)
            .selected()
            .map(crate::config::LanguageId::as_str),
        Some("bat")
    );
}

/// Through the pointer: a click on the drawn `[ Cancel ]` closes the form,
/// and one on `{ Yes }` answers the exit question.
#[test]
fn clicking_the_drawn_buttons_presses_them() {
    let machine = LspMachine::new();
    let mut app = machine.opened();
    let mut term = terminal();
    crate::ops::lsp_editor::open_form(&mut app, &FormTarget::New);
    click_text(&mut term, &mut app, "[ Cancel ]");
    assert_eq!(editor(&app).mode(), &Mode::Browse);

    crate::ops::lsp_editor::toggle(&mut app);
    editor_mut(&mut app).set_mode(Mode::ConfirmExit(ExitChoice::Save));
    click_text(&mut term, &mut app, "{ Yes }");
    assert!(matches!(app.overlay, Overlay::None), "saved and closed");
    assert!(machine.settings().exists());
}

/// A form whose Command field holds `py`, with a list of more than six
/// suggestions open under it, hanging over the form's button row.
fn form_with_a_long_list(machine: &LspMachine) -> App {
    for index in 0..9 {
        let file = machine
            .bin()
            .join(crate::lsp::program_file(&format!("pytool{index}")));
        crate::test_support::write_executable(&file, b"program").expect("program");
    }
    let mut app = machine.opened();
    crate::ops::lsp_editor::open_form(&mut app, &FormTarget::New);
    let form = editor_mut(&mut app).form_mut().expect("form");
    form.focus_on(FormField::Command);
    form.focused_line_mut()
        .expect("a text field")
        .set_split("py", "");
    crate::ops::lsp_editor::refresh_completion(
        &mut app,
        crate::ops::lsp_editor::Trigger::Requested,
    );
    assert!(
        editor_mut(&mut app)
            .form_mut()
            .expect("form")
            .completion()
            .is_some(),
        "the list is open"
    );
    app
}

/// The text drawn on screen row `row`.
fn screen_row(term: &Terminal<WritableTestBackend>, row: u16) -> String {
    let buffer = term.backend().buffer();
    (0..buffer.area.width)
        .map(|column| buffer[(column, row)].symbol())
        .collect()
}

/// The Command field's text, while the form is open.
fn command(app: &mut App) -> Option<String> {
    let form = editor_mut(app).form_mut()?;
    form.line(FormField::Command)
        .map(|line| line.text().to_owned())
}

/// The suggestion list is drawn over the form's buttons, and a click on it
/// must not press the hidden `{ OK }` or `[ Cancel ]` beneath (D11: list rows
/// take no click).
#[test]
fn a_click_on_the_suggestion_list_presses_no_button_beneath_it() {
    let machine = LspMachine::new();
    let mut app = form_with_a_long_list(&machine);
    let mut term = terminal();
    draw(&mut app, &mut term);
    let row = app.dialog_mouse.buttons_y;
    let screen = screen_row(&term, row);
    assert!(
        !screen.contains("OK"),
        "the list covers the buttons: {screen}"
    );
    for (_, left, _) in app.dialog_mouse.buttons.clone() {
        let press = MouseEventKind::Down(MouseButton::Left);
        gesture(&mut term, &mut app, press, (left + 1, row));
        assert_eq!(command(&mut app).as_deref(), Some("py"), "the form stays");
        draw(&mut app, &mut term);
    }
}

/// Press the left button on the form's `index`-th body row.
fn click_form_row(term: &mut Terminal<WritableTestBackend>, app: &mut App, index: u16) {
    draw(app, term);
    let body = app.dialog_mouse.body.expect("the form is drawn");
    let press = MouseEventKind::Down(MouseButton::Left);
    gesture(term, app, press, (body.x + 1, body.y + index));
}

fn form_focus(app: &mut App) -> FormField {
    editor_mut(app).form_mut().expect("form").focus()
}

/// As in the Copy and Name dialogs: a click on a field puts the cursor
/// there, one on Enabled toggles it, and the dimmed Language of an existing
/// server, the rule and the help row take no click.
#[test]
fn a_click_on_a_form_row_focuses_its_field_and_toggles_enabled() {
    let machine = LspMachine::new();
    let mut app = machine.opened();
    let css = crate::config::LanguageId::from_key("css");
    crate::ops::lsp_editor::open_form(&mut app, &FormTarget::Existing(css));
    let mut term = terminal();
    click_form_row(&mut term, &mut app, 3);
    assert_eq!(form_focus(&mut app), FormField::Extensions);
    let form = editor_mut(&mut app).form_mut().expect("form");
    assert_eq!(
        form.enabled(),
        crate::config::Enablement::Enabled,
        "only Enabled toggles"
    );
    click_form_row(&mut term, &mut app, 4);
    assert_eq!(form_focus(&mut app), FormField::Enabled);
    let form = editor_mut(&mut app).form_mut().expect("form");
    assert_eq!(form.enabled(), crate::config::Enablement::Disabled);
    for inert in [0, 5, 6] {
        click_form_row(&mut term, &mut app, inert);
        assert_eq!(form_focus(&mut app), FormField::Enabled, "row {inert}");
    }
}
