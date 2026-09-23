//! Purpose: F4 built-in editor entrypoint: loads the selected file as text and opens the editor overlay.

use std::io;

use crate::fileload::{read_for_text_ui, Loaded};
use crate::models::{App, Editor, FileSize, Overlay};
use crate::status::Status;
use crate::utils::MAX_FILE_SIZE;

/// F4 — open the selected file in the built-in Far Manager-style editor.
///
/// File-level problems (directory selected, not a regular file, too large,
/// binary, not valid text in the selected encoding) are reported via the
/// status line; `Err` is reserved for failures
/// outside the file itself, matching `handle_view`.
pub fn handle_edit_builtin(app: &mut App) -> io::Result<()> {
    if crate::ops::vault::edit_vault_if_kdbx(app) {
        return Ok(());
    }
    let Some(selected_entry) = app.get_active_panel().get_selected_entry() else {
        return Ok(());
    };

    if selected_entry.kind.is_dir() {
        app.set_status(Status::EditDirUnsupported);
        return Ok(());
    }

    let Some(selected) = app.get_active_panel().selected_path() else {
        return Ok(());
    };

    let editor = match load_editor(app, &selected) {
        Ok(editor) => editor,
        Err(status) => {
            app.set_status(status);
            return Ok(());
        }
    };

    app.set_status(Status::Editing(selected.display().to_string()));
    app.view_history.touch(&selected);
    app.set_overlay(Overlay::Editor(editor));
    Ok(())
}

/// The editor for `selected`, whose file held `bytes`: titled by its name,
/// carrying the encoding it was read with — or, for an empty file where
/// nothing can be sniffed, the one the settings name — and remembering what
/// the disk held, so a save can notice another program's write.
///
/// # Errors
///
/// When `bytes` are binary or not valid text in any encoding the editor can
/// write back unchanged (see [`editable_text`]).
pub fn editor_for(app: &App, selected: std::path::PathBuf, bytes: &[u8]) -> io::Result<Editor> {
    let (content, encoding) = editable_text(bytes)?;
    let title = selected
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let mut editor = Editor::from_content(title, selected, &content);
    editor.format.encoding = if content.is_empty() {
        configured_encoding(app)
    } else {
        encoding
    };
    editor.disk = Some(crate::models::Fingerprint::of(bytes));
    editor.tab_size =
        crate::models::TabSize::new(app.settings.number(crate::menu::SettingsPage::Editor));
    Ok(editor)
}

fn configured_encoding(app: &App) -> crate::encoding::TextEncoding {
    use crate::encoding::TextEncoding as Encoding;
    match app.settings.get(crate::menu::SettingsPage::CodePages) {
        "UTF-16LE" => Encoding::Utf16Le,
        "UTF-16BE" => Encoding::Utf16Be,
        "Windows-1251" => Encoding::Windows1251,
        "Windows-1252" => Encoding::Windows1252,
        _ => Encoding::Utf8,
    }
}

/// The selected file in an editor, or the status explaining why it cannot be.
///
/// Shares [`read_for_text_ui`] with the F3 viewer, so the "not a regular file"
/// and "too large" guards can never drift apart between F3 and F4. Without the
/// first one, F4 on a writer-less FIFO blocks the event loop in `File::open`
/// forever and F4 on /dev/zero reads until memory runs out — both report
/// `len() == 0`, so a size check by itself never even saw them.
fn load_editor(app: &App, path: &std::path::Path) -> Result<Editor, Status> {
    let cannot_edit = |error: String| Status::CannotEdit {
        path: path.to_path_buf(),
        error,
    };
    match read_for_text_ui(path) {
        Ok(Loaded::Bytes(bytes)) => {
            editor_for(app, path.to_path_buf(), &bytes).map_err(|err| cannot_edit(err.to_string()))
        }
        Ok(Loaded::NotRegular) => Err(Status::NotRegularFile(path.display().to_string())),
        Ok(Loaded::TooLarge(size)) => Err(Status::FileTooLargeEdit {
            megabytes: FileSize::from_bytes(size).megabytes(),
            limit_megabytes: MAX_FILE_SIZE.megabytes(),
        }),
        Err(err) => Err(cannot_edit(err.to_string())),
    }
}

/// `bytes` decoded for editing, with the encoding they were read in, or why
/// the editor must not take them.
///
/// The editor writes what it loaded straight back, so unlike the read-only
/// viewer it may not decode lossily: that would replace every undecodable
/// byte with U+FFFD and corrupt the file on save. A decoded NUL is the binary
/// heuristic F3 and Quick view use; it is looked for *after* decoding, so
/// UTF-16 text, whose BOM accounts for its NUL bytes, still opens.
fn editable_text(bytes: &[u8]) -> io::Result<(String, crate::encoding::TextEncoding)> {
    let encoding = crate::encoding::TextEncoding::detect(bytes);
    let text = encoding.decode(bytes)?;
    if text.contains('\0') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "binary file (it contains NUL bytes)",
        ));
    }
    Ok((text, encoding))
}
