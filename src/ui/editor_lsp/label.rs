//! Editor labels retain the file name when the full path cannot fit.

use std::path::Path;

use crate::{
    models::Editor,
    ui::{printable_name, Cells},
};

#[cfg(test)]
pub(in crate::ui) fn label(editor: &Editor, marker: &str) -> String {
    for_path(&super::status(editor), marker, &editor.path)
}

pub(in crate::ui) fn fitted_label(editor: &Editor, marker: &str, room: Cells) -> String {
    let status = super::status(editor);
    let full = for_path(&status, marker, &editor.path);
    if Cells::of(&full) > room {
        let compact = editor
            .path
            .file_name()
            .map_or(editor.path.as_path(), Path::new);
        for_path(&status, marker, compact)
    } else {
        full
    }
}

fn for_path(status: &str, marker: &str, path: &Path) -> String {
    let raw = crate::path_display::label(path);
    let path = printable_name(&raw);
    if status.is_empty() {
        format!(" {path}{marker}")
    } else {
        format!(" {} | {path}{marker}", status.trim_start_matches(" | "))
    }
}
