//! Purpose: Draws the Describe files dialog (Ctrl+Z): the entry's name, one
//! text field, and OK / Cancel.

use ratatui::Frame;

use super::{dialog, Cells};

/// Far's own title for the dialog.
const TITLE: &str = "Describe files";

pub(super) fn render(
    f: &mut Frame,
    tracker: &mut crate::mouse::DialogMouse,
    dialog: &crate::models::DescribePrompt,
) {
    // The name is in the caption because the dialog comes up once per marked
    // entry: without it, the third prompt in a row is a guess. A label reads
    // `&` as a hotkey marker, so one in the name is doubled to survive.
    let caption = format!(
        "Enter description for {}:",
        dialog.name().replace('&', "&&")
    );
    let editing = !dialog.on_cancel();
    // No history marker: the field remembers nothing, and a `↓` would
    // promise a list that never drops down.
    let rows = [
        dialog::Row::Label(&caption),
        dialog::Row::input(dialog.text(), editing, false),
    ];
    let buttons = [
        dialog::Button::new("&OK").default_button().focused(editing),
        dialog::Button::new("&Cancel").focused(!editing),
    ];
    if let Some((x, y)) =
        dialog::render_tracked(f, tracker, TITLE, dialog::Ground::Prompt, &rows, &buttons)
    {
        f.set_cursor_position((caret_column(x, dialog.after_caret()), y));
    }
}

/// The dialog places the caret after the text; the prompt's own cursor may
/// stand further left, by the width of what follows it.
fn caret_column(end: u16, after_caret: &str) -> u16 {
    let back = u16::try_from(Cells::of(after_caret).get()).unwrap_or(u16::MAX);
    end.saturating_sub(back)
}

#[cfg(test)]
mod tests;
