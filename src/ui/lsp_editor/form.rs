//! Purpose: The Language servers edit form and exit question, drawn as Far's
//! dialogs over the list, with the suggestion list hanging from its field.

use ratatui::{layout::Rect, Frame};

use super::super::completion::{render_dropdown, Highlight, Side};
use super::super::dialog::{self, Button, Ground, Row};
use super::super::{head_cells, tail_cells, take_cells, Cells};
use crate::lineedit::LineEditor;
use crate::models::lsp_editor::{FormField, FormTarget, ListFocus, ServerForm};
use crate::models::ExitChoice;
use crate::mouse::DialogMouse;

/// The text controls, their captions, and their rows in the dialog body.
const TEXT_FIELDS: [(FormField, &str); 4] = [
    (FormField::Language, "Language:"),
    (FormField::Command, "Command:"),
    (FormField::Arguments, "Arguments:"),
    (FormField::Extensions, "Extensions:"),
];

pub(super) fn render(f: &mut Frame<'_>, tracker: &mut DialogMouse, form: &ServerForm) {
    let title = match form.target() {
        FormTarget::Existing(language) => format!("Edit server: {language}"),
        FormTarget::New => "Add server".to_owned(),
    };
    let help = form.help();
    let buttons = buttons(form.focus());
    // The values do not change the dialog's size, so it is placed before
    // they are cut to the room their boxes have.
    let blank: [String; 4] = Default::default();
    let sizing = rows(form, &help, &blank);
    let Some(placement) = dialog::place(f.area(), &title, &sizing, &buttons, tracker.offset) else {
        tracker.clear();
        return;
    };
    let room = field_room(placement.body.width);
    let shown = TEXT_FIELDS.map(|(field, _)| shown(form, field, room));
    let values = shown.clone().map(|(text, _)| text);
    let body = rows(form, &help, &values);
    dialog::render_tracked(f, tracker, &title, Ground::Prompt, &body, &buttons);
    let focused = TEXT_FIELDS
        .iter()
        .zip(&shown)
        .enumerate()
        .find(|(_, ((field, _), _))| *field == form.focus());
    if let Some((index, ((_, caption), (_, caret)))) = focused {
        let y = placement.body.y + u16::try_from(index).unwrap_or(u16::MAX);
        let at = Rect::new(placement.body.x, y, room_u16(room), 1);
        tracker.covered = caret_and_list(f, form, at, caption, caret);
    }
}

/// The caret in the focused field on row `at`, and its suggestion list
/// hanging below it; returns where the list was drawn.
fn caret_and_list(
    f: &mut Frame<'_>,
    form: &ServerForm,
    at: Rect,
    caption: &str,
    caret: &str,
) -> Option<Rect> {
    f.set_cursor_position((at.x + dialog::field_caret(caption, caret), at.y));
    let list = form.completion()?;
    let x = at.x + dialog::field_caret(caption, "");
    let anchor = Rect::new(x, at.y, at.width.saturating_add(1), 1);
    let highlight = match list.focus() {
        ListFocus::List => Highlight::Selected,
        ListFocus::Field => Highlight::None,
    };
    render_dropdown(f, list.menu(), anchor, Side::Below, highlight)
}

fn room_u16(room: Cells) -> u16 {
    u16::try_from(room.get()).unwrap_or(u16::MAX)
}

/// The cells a field's box holds in a body `width` cells wide, as
/// `dialog::captioned_box` sizes it.
fn field_room(width: u16) -> Cells {
    let taken = usize::from(dialog::field_caret("", ""));
    let caption = TEXT_FIELDS
        .iter()
        .map(|(_, caption)| usize::from(dialog::field_caret(caption, "")))
        .max()
        .unwrap_or(taken);
    Cells::new(usize::from(width).saturating_sub(caption + 1))
}

/// The part of `field`'s text its box shows, and the part before the caret.
///
/// The focused field scrolls to keep the caret in view, as the command
/// prompt does; another field shows its head, cut with `…`.
fn shown(form: &ServerForm, field: FormField, room: Cells) -> (String, String) {
    let Some(line) = form.line(field) else {
        return (String::new(), String::new());
    };
    if field != form.focus() {
        return (head_cells(line.text(), room), String::new());
    }
    window(line, room)
}

fn window(line: &LineEditor, room: Cells) -> (String, String) {
    let before = tail_cells(line.before_cursor(), room.saturating_sub(Cells::new(1)));
    let after_room = room.saturating_sub(Cells::of(&before));
    let (after, _) = take_cells(line.after_cursor(), after_room);
    (format!("{before}{after}"), before)
}

fn rows<'a>(form: &ServerForm, help: &'a str, values: &'a [String; 4]) -> Vec<Row<'a>> {
    let mut rows: Vec<Row<'_>> = TEXT_FIELDS
        .iter()
        .zip(values)
        .map(|((field, caption), value)| Row::Field {
            caption,
            value,
            focused: form.focus() == *field,
            enabled: field.allows(form.target()),
            // The form scrolls a long value itself and places its caret.
            edit: None,
        })
        .collect();
    rows.push(Row::Checkbox {
        label: "Enabled",
        checked: form.enabled() == crate::config::Enablement::Enabled,
        focused: form.focus() == FormField::Enabled,
        enabled: true,
    });
    rows.push(Row::Divider);
    rows.push(Row::Label(help));
    rows
}

fn buttons(focus: FormField) -> [Button<'static>; 2] {
    [
        Button::new("OK")
            .default_button()
            .focused(focus == FormField::Ok),
        Button::new("Cancel").focused(focus == FormField::Cancel),
    ]
}

/// The exit question's keys, in the bottom border behind it.
pub(super) const EXIT_LEGEND: &str = "Y Yes  N No  Esc Cancel";

/// The form's keys that are not F-keys, for the bottom border: with a list
/// open, what Up, Down, Enter and Esc do to it.
pub(super) const fn legend(form: &ServerForm) -> &'static str {
    match form.completion() {
        None => "Enter OK  Esc Cancel  Tab Next  Ctrl+Space List",
        Some(list) => match list.focus() {
            ListFocus::Field => "Down Choose  Enter OK  Esc Close list",
            ListFocus::List => "Up Down Choose  Enter Insert  Esc Close list",
        },
    }
}

/// Esc with unsaved changes: the editor's question, on its red ground.
pub(super) fn render_exit_question(
    f: &mut Frame<'_>,
    tracker: &mut DialogMouse,
    choice: ExitChoice,
) {
    dialog::render_warning(
        f,
        tracker,
        "Language servers",
        &["Language server settings have been modified. Save?".to_owned()],
        &[
            Button::new("Yes")
                .default_button()
                .focused(choice == ExitChoice::Save),
            Button::new("No").focused(choice == ExitChoice::Discard),
            Button::new("Cancel").focused(choice == ExitChoice::Cancel),
        ],
    );
}

#[cfg(test)]
mod tests_edges;
