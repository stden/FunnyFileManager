//! Purpose: The file panel filter dialog (Ctrl+I).

use ratatui::Frame;

use super::dialog;
use crate::models::{FilterField, FilterInput, FilterPrompt};

pub(super) fn render(
    f: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    prompt: &FilterPrompt,
    input: &FilterInput,
) {
    let rows = rows(prompt, input);
    let buttons = [
        dialog::Button::new("&Apply")
            .default_button()
            .focused(input.field == FilterField::Apply),
        dialog::Button::new("&Cancel").focused(input.field == FilterField::Cancel),
    ];
    if let Some(caret) = dialog::render_tracked(
        f,
        tracker,
        "File panel filter",
        dialog::Ground::Prompt,
        &rows,
        &buttons,
    ) {
        f.set_cursor_position(caret);
    }
}

fn rows<'a>(prompt: &FilterPrompt, input: &'a FilterInput) -> Vec<dialog::Row<'a>> {
    let mut rows = fields(input);
    rows.push(dialog::Row::Checkbox {
        label: "Hide &dotfiles",
        checked: prompt.hide_dotfiles,
        focused: input.field == FilterField::Hidden,
        enabled: true,
    });
    if let Some(error) = &input.error {
        rows.push(dialog::Row::Plain(error));
    }
    rows
}

fn fields(input: &FilterInput) -> Vec<dialog::Row<'_>> {
    let mut rows = vec![dialog::Row::field(
        "&Include:",
        &input.include,
        input.field == FilterField::Include,
        true,
    )];
    if let Some(additional) = &input.additional {
        rows.push(dialog::Row::field(
            "E&xtra include:",
            additional,
            input.field == FilterField::Additional,
            true,
        ));
    }
    rows.push(dialog::Row::field(
        "&Exclude:",
        &input.exclude,
        input.field == FilterField::Exclude,
        true,
    ));
    rows
}
