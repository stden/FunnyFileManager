//! Purpose: Route pasted text to the focused modal field without exposing panel input.
use super::{command_line, editor, filter, remote_dialog, tasks, text_field};
use crate::models::{App, Overlay};
/// A terminal paste: text for the editor's document, for the task list's open
/// bar, or for the text field that has the cursor in any other dialog — as one
/// line, over its selection.
pub fn paste_text(app: &mut App, text: &str) {
    match app.overlay {
        Overlay::Editor(_) => editor::paste_text(app, text),
        Overlay::Tasks(_) => tasks::paste(app, text),
        Overlay::CommandLine(_) => command_line::paste(app, text),
        Overlay::Remote(_) => remote_dialog::paste(app, text),
        Overlay::Filter(_) => filter::paste(app, text),
        _ => paste_plain(app, text),
    }
}

/// One line into the focused editor, or into a dialog whose fields are strings.
fn paste_plain(app: &mut App, text: &str) {
    let line = text_field::one_line(text);
    if let Some(field) = app.focused_text_field() {
        field.insert_str(&line);
        if matches!(app.overlay, Overlay::LspEditor(_)) {
            crate::ops::lsp_editor::refresh_completion(app, crate::ops::lsp_editor::Trigger::Typed);
        }
        return;
    }
    match &mut app.overlay {
        Overlay::SessionEdit(dialog) => dialog.insert_text(&line),
        Overlay::Attributes(dialog) => dialog.insert_text(&line),
        _ => {}
    }
}
