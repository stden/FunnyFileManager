//! Purpose: Draws the Add to archive dialog (Shift+F1): the name field, the
//! format radio, the delete checkbox and the Add / Cancel buttons.
//!
//! Reads the dialog's state and nothing else; which formats the radio offers
//! was decided when the dialog opened, by the code that may look at the PATH.

use ratatui::Frame;

use super::dialog;
use crate::models::{ArchiveField, ArchivePrompt};

pub(super) fn render(
    f: &mut Frame,
    tracker: &mut crate::mouse::DialogMouse,
    dialog: &ArchivePrompt,
) {
    let caption = dialog.caption();
    let formats: Vec<&str> = dialog.offered.iter().map(|format| format.label()).collect();
    let on = |field: ArchiveField| dialog.field == field;
    let rows = rows(dialog, &caption, &formats);
    let buttons = [
        dialog::Button::new("&Add")
            .default_button()
            .focused(on(ArchiveField::Add)),
        dialog::Button::new("&Cancel").focused(on(ArchiveField::Cancel)),
    ];
    if let Some((x, y)) = dialog::render_tracked(
        f,
        tracker,
        "Add to archive",
        dialog::Ground::Prompt,
        &rows,
        &buttons,
    ) {
        f.set_cursor_position((x, y));
    }
}

/// The caption, the name, the radio and the checkbox, top to bottom.
fn rows<'a>(
    dialog: &'a ArchivePrompt,
    caption: &'a str,
    formats: &'a [&'a str],
) -> [dialog::Row<'a>; 6] {
    let on = |field: ArchiveField| dialog.field == field;
    [
        dialog::Row::Label(caption),
        dialog::Row::input(&dialog.name, on(ArchiveField::Name), true),
        dialog::Row::Divider,
        dialog::Row::Radio {
            caption: "&Format:",
            options: formats,
            selected: dialog.format_index(),
            focused: on(ArchiveField::Format),
        },
        dialog::Row::Divider,
        dialog::Row::checkbox(
            "&Delete files after archiving",
            dialog.delete_after,
            on(ArchiveField::DeleteAfter),
        ),
    ]
}

#[cfg(test)]
mod tests;
