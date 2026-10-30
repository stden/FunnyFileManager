//! Purpose: Draws the Describe files dialog (Ctrl+Z): the entry's name, one
//! text field, and OK / Cancel.

use ratatui::Frame;

use super::dialog;

/// Far's own title for the dialog.
const TITLE: &str = "Describe files";

pub(super) fn render(
    f: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    dialog: &crate::models::DescribePrompt,
) {
    // The name is in the caption because the dialog comes up once per marked
    // entry: without it, the third prompt in a row is a guess. A label reads
    // `&` as a hotkey marker, so one in the name is doubled to survive.
    let caption = format!(
        "Enter description for {}:",
        crate::ui::printable_name(dialog.name()).replace('&', "&&")
    );
    let editing = !dialog.on_cancel();
    // No history marker: the field remembers nothing, and a `↓` would
    // promise a list that never drops down.
    let rows = [
        dialog::Row::Label(&caption),
        dialog::Row::line(dialog.line(), editing, false),
    ];
    let buttons = [
        dialog::Button::new("&OK").default_button().focused(editing),
        dialog::Button::new("&Cancel").focused(!editing),
    ];
    if let Some(caret) =
        dialog::render_tracked(f, tracker, TITLE, dialog::Ground::Prompt, &rows, &buttons)
    {
        f.set_cursor_position(caret);
    }
}

#[cfg(test)]
mod tests;
