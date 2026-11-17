//! Purpose: Pin the form on screen: its fields, the dimmed language of an
//! existing server, the caret in a scrolled field, and the dropdown.

use super::*;
use crate::models::lsp_editor::FormField;

pub(super) fn form_on(app: &mut App, language: &str) {
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
    assert!(buffer[(x, y)].modifier.contains(Modifier::ITALIC));
    let (x, y) = find(&buffer, "Command:");
    assert!(!buffer[(x, y)].modifier.contains(Modifier::ITALIC));
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
