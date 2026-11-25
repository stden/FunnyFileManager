//! Purpose: Draws the Apply command dialog (Ctrl+G): Far's one-field box, the
//! command typed so far, and OK / Cancel.

use ratatui::Frame;

use super::dialog;

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
            dialog::Row::line(dialog.input(), on_ok, true),
            dialog::Row::Text(UNESCAPED_WARNING[0]),
            dialog::Row::Text(UNESCAPED_WARNING[1]),
        ],
        &[
            dialog::Button::new("&OK").default_button().focused(on_ok),
            dialog::Button::new("&Cancel").focused(!on_ok),
        ],
    );
    if let Some(caret) = caret {
        f.set_cursor_position(caret);
    }
}

#[cfg(test)]
mod tests;
