//! Clipboard use cases for the built-in editor. Backends are injected on App.

use crate::clipboard::{copy_secret, CopyRoute};
use crate::models::App;

pub fn copy_editor_selection(app: &mut App) {
    let Some(text) = app
        .get_editor()
        .and_then(crate::models::Editor::selected_text)
    else {
        return;
    };
    let notice = copy(app, &text, "Selection copied");
    if let Some(editor) = app.get_editor_mut() {
        editor.notice = Some(notice);
    }
}

/// Ctrl+F: the edited file's full name goes into the text at the cursor,
/// replacing a selection as a paste would, and onto the clipboard. A
/// clipboard that fails is reported; the name is typed all the same.
pub fn insert_editor_file_name(app: &mut App) {
    let Some(name) = app
        .get_editor()
        .map(|editor| crate::path_display::label(&editor.path).into_owned())
    else {
        return;
    };
    let notice = copy(app, &name, "File name copied");
    if let Some(editor) = app.get_editor_mut() {
        editor.insert_text(&name);
        editor.notice = Some(notice);
    }
}

/// Put `text` on the clipboard, and say how it went: `copied` when it is on
/// the OS clipboard.
fn copy(app: &mut App, text: &str, copied: &str) -> String {
    let result = app
        .clipboard
        .backend_mut()
        .ok_or_else(|| "no clipboard available".to_owned())
        .and_then(|backend| copy_secret(backend, text));
    match result {
        Ok(route) => {
            app.clipboard.disarm();
            match route {
                CopyRoute::Native => copied.to_owned(),
                CopyRoute::Osc52 => "Copy sent to terminal clipboard".to_owned(),
            }
        }
        Err(error) => format!("Copy failed: {error}"),
    }
}

pub fn paste_into_editor(app: &mut App) {
    if app.get_editor().is_none() {
        return;
    }
    let result = app
        .clipboard
        .backend_mut()
        .ok_or_else(|| "no clipboard available".to_owned())
        .and_then(crate::clipboard::ClipboardBackend::get_native);
    if let Some(editor) = app.get_editor_mut() {
        match result {
            Ok(text) => editor.insert_text(&text),
            Err(_) => {
                editor.notice = Some("Paste failed: clipboard text is unavailable".to_owned());
            }
        }
    }
}
