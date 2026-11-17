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
    #[cfg(windows)]
    if is_unc(&target.path) && !is_unc(&editor.path) {
        return Err("Cannot read definition target");
    }
    open_target(app, target)
}

fn open_target(app: &mut App, target: &Definition) -> Result<(), &'static str> {
    #[expect(
        clippy::map_err_ignore,
        reason = "the editor notice is one fixed line per failure; the loader's detail is not shown there"
    )]
    let loaded = crate::fileload::read_for_text_ui(&target.path)
        .map_err(|_| "Cannot read definition target")?;
    let crate::fileload::Loaded::Bytes(bytes) = loaded else {
        return Err("Definition target is not an editable text file");
    };
    #[expect(
        clippy::map_err_ignore,
        reason = "the editor notice is one fixed line per failure; the editor's detail is not shown there"
    )]
    let mut editor = crate::editor::editor_for(app, target.path.clone(), &bytes)
        .map_err(|_| "Definition target is not an editable text file")?;
    move_cursor(&mut editor, target)?;
    app.set_overlay(Overlay::Editor(editor));
    Ok(())
}

fn move_cursor(editor: &mut Editor, target: &Definition) -> Result<(), &'static str> {
    let position = target.range.start;
    // A line number `usize` cannot hold is past the end of any file.
    let line_index = usize::try_from(position.line).unwrap_or(usize::MAX);
    let line = editor
        .lines
        .get(line_index)
        .ok_or("Definition range is outside the file")?;
    let column =
        positions::to_chars(line, position.character).ok_or("Invalid definition position")?;
    editor.cursor_line = line_index;
    editor.cursor_col = column;
    editor.selection = crate::models::EditorSelection::None;
    editor.lsp.popup = None;
    Ok(())
}

#[cfg(windows)]
fn is_unc(path: &std::path::Path) -> bool {
    matches!(
        path.components().next(),
        Some(std::path::Component::Prefix(p))
            if matches!(
                p.kind(),
                std::path::Prefix::UNC(..) | std::path::Prefix::VerbatimUNC(..)
            )
    )
}

#[cfg(test)]
mod tests;
