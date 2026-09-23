//! Purpose: Draws the Extract files dialog (Shift+F2): what is being unpacked,
//! the destination field, and Far's Extract / Cancel buttons.
//!
//! The caption carries the honesty: with nothing selected that is an archive
//! it lists the kinds that are, and the Extract button is dimmed rather than
//! hidden, so the dialog keeps its shape and the user learns where the button
//! will be when it applies.

use ratatui::Frame;

use super::dialog::{self, Button, Ground, Row};
use crate::models::{ExtractField, ExtractPrompt};

pub(super) fn render(
    f: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    dialog: &ExtractPrompt,
) {
    let caption = dialog.caption();
    let rows = [
        Row::Label(&caption),
        Row::input(
            &dialog.destination,
            dialog.field == ExtractField::Destination,
            true,
        ),
    ];
    let buttons = [
        Button::new("&Extract")
            .default_button()
            .focused(dialog.field == ExtractField::Extract)
            .enabled(dialog.can_extract()),
        Button::new("&Cancel").focused(dialog.field == ExtractField::Cancel),
    ];
    if let Some((x, y)) =
        dialog::render_tracked(f, tracker, "Extract files", Ground::Prompt, &rows, &buttons)
    {
        f.set_cursor_position((x, y));
    }
}

#[cfg(test)]
mod tests;
