//! Editor labels retain the file name when the full path cannot fit.

use std::path::Path;

use crate::{models::Editor, ui::printable_name};

pub(in crate::ui) fn label(editor: &Editor, marker: &str) -> String {
    for_path(editor, marker, &editor.path)
}

pub(in crate::ui) fn compact_label(editor: &Editor, marker: &str) -> String {
    let path = editor
        .path
        .file_name()
        .map_or(editor.path.as_path(), Path::new);
    for_path(editor, marker, path)
}

fn for_path(editor: &Editor, marker: &str, path: &Path) -> String {
    let raw = crate::path_display::label(path);
    let path = printable_name(&raw);
    let status = super::status(editor);
    if status.is_empty() {
        format!(" {path}{marker}")
    } else {
        format!(" {} | {path}{marker}", status.trim_start_matches(" | "))
    }
}
