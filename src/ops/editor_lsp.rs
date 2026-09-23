//! Purpose: Open definition targets through the bounded text loader, preserving unsaved edits.

use crate::{
    lsp::{features::Definition, positions},
    models::{App, Editor, Overlay},
};

pub fn definition(app: &mut App, target: &Definition) {
    if let Err(message) = navigate(app, target) {
        if let Some(editor) = app.get_editor_mut() {
            editor.notice = Some(message.into());
        }
    }
}

fn navigate(app: &mut App, target: &Definition) -> Result<(), &'static str> {
    let Some(editor) = app.get_editor_mut() else {
        return Ok(());
    };
    if crate::lsp::uri::same_path(&editor.path, &target.path) {
        return move_cursor(editor, target);
    }
    if editor.state.is_modified() {
        return Err("Save this file with F2 before opening a definition in another file");
    }
    let loaded = crate::fileload::read_for_text_ui(&target.path)
        .map_err(|_| "Cannot read definition target")?;
    let crate::fileload::Loaded::Bytes(bytes) = loaded else {
        return Err("Definition target is not an editable text file");
    };
    let encoding = crate::encoding::TextEncoding::detect(&bytes);
    let text = encoding
        .decode(&bytes)
        .map_err(|_| "Cannot decode definition target")?;
    let mut editor = crate::editor::editor_for(app, target.path.clone(), &text, encoding);
    move_cursor(&mut editor, target)?;
    app.set_overlay(Overlay::Editor(editor));
    Ok(())
}

fn move_cursor(editor: &mut Editor, target: &Definition) -> Result<(), &'static str> {
    let position = target.range.start;
    let line = editor
        .lines
        .get(position.line as usize)
        .ok_or("Definition range is outside the file")?;
    let column =
        positions::to_chars(line, position.character).ok_or("Invalid definition position")?;
    editor.cursor_line = position.line as usize;
    editor.cursor_col = column;
    editor.selection = crate::models::EditorSelection::None;
    editor.lsp.popup = None;
    Ok(())
}

#[cfg(test)]
mod tests;
