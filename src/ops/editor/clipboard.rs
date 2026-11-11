//! Clipboard use cases for the built-in editor. Backends are injected on App.

use crate::clipboard::{copy_secret, CopyRoute};
use crate::models::{App, ClipboardSlot, Overlay};

pub fn copy_editor_selection(app: &mut App) {
    let Overlay::Editor(editor) = &mut app.overlay else {
        return;
    };
    let Some(text) = editor.selected_text() else {
        return;
    };
    editor.notice = Some(
        match try_copy(&mut app.clipboard, &text, "Selection copied") {
            Ok(notice) => notice,
            Err(error) => format!("Copy failed: {error}"),
        },
    );
}

pub fn cut_editor_selection(app: &mut App) {
    let Overlay::Editor(editor) = &mut app.overlay else {
        return;
    };
    let Some(text) = editor.selected_text() else {
        return;
    };
    match try_copy(&mut app.clipboard, &text, "Selection cut") {
        Ok(notice) => {
            let cut_all = editor.selection == crate::models::EditorSelection::All;
            editor.delete_selection();
            if cut_all {
                // `SelectAll` puts the file's final newline on the clipboard too,
                // so nothing of it stays behind in the emptied buffer.
                editor.format.trailing_newline = false;
            }
            editor.notice = Some(notice);
        }
        Err(error) => {
            editor.notice = Some(format!("Cut failed: {error}"));
        }
    }
}

/// Ctrl+F: the edited file's full name goes into the text at the cursor,
/// replacing a selection as a paste would, and onto the clipboard. A
/// clipboard that fails is reported; the name is typed all the same.
pub fn insert_editor_file_name(app: &mut App) {
    let Overlay::Editor(editor) = &mut app.overlay else {
        return;
    };
    let name = crate::path_display::label(&editor.path).into_owned();
    let notice = match try_copy(&mut app.clipboard, &name, "File name copied") {
        Ok(notice) => notice,
        Err(error) => format!("Copy failed: {error}"),
    };
    editor.insert_text(&name);
    editor.notice = Some(notice);
}

/// Put `text` on the clipboard, and say how it went: `copied` when it is on
/// the OS clipboard, or a terminal notice when OSC 52 carried it.
fn try_copy(clipboard: &mut ClipboardSlot, text: &str, copied: &str) -> Result<String, String> {
    let route = clipboard
        .backend_mut()
        .ok_or_else(|| "no clipboard available".to_owned())
        .and_then(|backend| copy_secret(backend, text))?;
    clipboard.disarm();
    Ok(match route {
        CopyRoute::Native => copied.to_owned(),
        CopyRoute::Osc52 => "Copy sent to terminal clipboard".to_owned(),
    })
}

pub fn paste_into_editor(app: &mut App) {
    let Overlay::Editor(editor) = &mut app.overlay else {
        return;
    };
    let result = app
        .clipboard
        .backend_mut()
        .ok_or_else(|| "no clipboard available".to_owned())
        .and_then(crate::clipboard::ClipboardBackend::get_native);
    match result {
        Ok(text) => editor.insert_text(&text),
        Err(_) => {
            editor.notice = Some("Paste failed: clipboard text is unavailable".to_owned());
        }
    }
}
