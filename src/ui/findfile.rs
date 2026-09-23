//! Purpose: Draws the Find file dialog (Alt+F7): name mask and containing text.

use ratatui::Frame;

use super::dialog;
use crate::models::{FindFile, FindFileField};

pub(super) fn render(
    f: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    dialog: &FindFile,
) {
    let on = |field| dialog.field() == field;
    let rows = [
        dialog::Row::Label("&File masks"),
        dialog::Row::input(&dialog.mask, on(FindFileField::Mask), false),
        dialog::Row::Label("&Containing text"),
        dialog::Row::input(&dialog.containing, on(FindFileField::Containing), false),
    ];
    let buttons = [
        dialog::Button::new("&Find")
            .default_button()
            .focused(on(FindFileField::Find)),
        dialog::Button::new("&Cancel").focused(on(FindFileField::Cancel)),
    ];
    if let Some((x, y)) = dialog::render_tracked(
        f,
        tracker,
        "Find file",
        dialog::Ground::Prompt,
        &rows,
        &buttons,
    ) {
        if !dialog.on_button() {
            f.set_cursor_position((x, y));
        }
    }
}

#[cfg(test)]
mod tests;
