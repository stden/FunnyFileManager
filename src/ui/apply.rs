//! Purpose: Draws the Apply command dialog (Ctrl+G): Far's one-field box, the
//! command typed so far, and OK / Cancel.

use ratatui::Frame;

use super::{dialog, Cells};

/// Far's behaviour, kept on purpose: `!.!` and friends are pasted in as they
/// are and the platform shell parses the line, so a file named `$(rm x)` or
/// `a;b` runs code. The dialog says so where the command is typed.
const UNESCAPED_WARNING: [&str; 2] = [
    "Warning: names are inserted unescaped and run by the shell;",
    "a name containing ; $ ` or quotes can run commands.",
];

pub(super) fn render(
    f: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    dialog: &crate::models::ApplyPrompt,
) {
    // Far's own box: the field remembers earlier commands (the `↓` marker),
    // and OK is the default so Enter in the field runs what was typed.
    let on_ok = !dialog.on_cancel();
    let caret = dialog::render_tracked(
        f,
        tracker,
        "Apply command",
        dialog::Ground::Prompt,
        &[
            dialog::Row::Label("Enter &command:"),
            dialog::Row::input(dialog.command(), on_ok, true),
            dialog::Row::Text(UNESCAPED_WARNING[0]),
            dialog::Row::Text(UNESCAPED_WARNING[1]),
        ],
        &[
            dialog::Button::new("&OK").default_button().focused(on_ok),
            dialog::Button::new("&Cancel").focused(!on_ok),
        ],
    );
    if let Some((end, y)) = caret {
        // The box reports the end of the text; the caret sits wherever the
        // editor's cursor is, which may be mid-line after Left or Home.
        let after = u16::try_from(Cells::of(dialog.input().after_cursor()).get()).unwrap_or(0);
        f.set_cursor_position((end.saturating_sub(after), y));
    }
}

#[cfg(test)]
mod tests;
