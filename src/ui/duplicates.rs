//! Purpose: Draws the Find duplicate blocks dialog: the folder, the file mask,
//! the block size and the number of groups, then Start and Cancel.

use ratatui::Frame;

use super::dialog;
use crate::models::{Duplicates, DuplicatesField};

pub(super) fn render(
    f: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    dialog: &Duplicates,
) {
    let on = |field| dialog.field() == field;
    let rows = [
        dialog::Row::Label("&Folder"),
        dialog::Row::line(&dialog.folder, on(DuplicatesField::Folder), false),
        dialog::Row::Label("File &mask"),
        dialog::Row::line(&dialog.mask, on(DuplicatesField::Mask), false),
        dialog::Row::Label("Minimum block, &lines"),
        dialog::Row::line(&dialog.min_block, on(DuplicatesField::MinBlock), false),
        dialog::Row::Label("&Groups to show"),
        dialog::Row::line(&dialog.groups, on(DuplicatesField::Groups), false),
    ];
    let buttons = [
        dialog::Button::new("&Start")
            .default_button()
            .focused(on(DuplicatesField::Start)),
        dialog::Button::new("&Cancel").focused(on(DuplicatesField::Cancel)),
    ];
    if let Some((x, y)) = dialog::render_tracked(
        f,
        tracker,
        "Find duplicate blocks",
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
