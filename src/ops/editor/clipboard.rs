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
    let result = app
        .clipboard
        .backend_mut()
        .ok_or_else(|| "no clipboard available".to_owned())
        .and_then(|backend| copy_secret(backend, &text));
    let notice = match result {
        Ok(route) => {
            app.clipboard.disarm();
            match route {
                CopyRoute::Native => "Selection copied".to_owned(),
                CopyRoute::Osc52 => "Copy sent to terminal clipboard".to_owned(),
            }
        }
        Err(error) => format!("Copy failed: {error}"),
    };
    if let Some(editor) = app.get_editor_mut() {
        editor.notice = Some(notice);
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
