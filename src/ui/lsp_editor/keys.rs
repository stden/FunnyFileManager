//! Purpose: The Language servers dialog's keys on screen: its own F-key bar,
//! with a label only where the key does something, and the legend of the
//! other keys written into the bottom border.

use super::super::fkey_bar::KeyLabels;
use super::{form, list};
use crate::models::lsp_editor::{Access, Mode};
use crate::models::LspEditor;

/// The list's key bar; a read-only dialog blanks the keys that change it.
const LIST_KEYS: KeyLabels<'static> = [
    ("1", ""),
    ("2", "Save"),
    ("3", "View"),
    ("4", "Edit"),
    ("5", "Check"),
    ("6", ""),
    ("7", "Add"),
    ("8", "Reset"),
    ("9", ""),
    ("10", "Quit"),
    ("11", ""),
    ("12", ""),
];

/// The F3 view's key bar.
const VIEW_KEYS: KeyLabels<'static> = [
    ("1", ""),
    ("2", ""),
    ("3", "List"),
    ("4", ""),
    ("5", "Check"),
    ("6", ""),
    ("7", ""),
    ("8", ""),
    ("9", ""),
    ("10", "Back"),
    ("11", ""),
    ("12", ""),
];

/// The keys that are not F-keys, written into the bottom border, for what
/// is on screen; the F3 view writes its own.
pub(super) fn legend(editor: &LspEditor) -> String {
    match editor.mode() {
        Mode::Form(dialog) => form::legend(dialog).to_owned(),
        Mode::ConfirmExit(_) => form::EXIT_LEGEND.to_owned(),
        Mode::Browse | Mode::View(_) => list::legend(editor),
    }
}

/// The dialog's own key bar: a label only where the key does something.
pub(super) fn key_labels(editor: &LspEditor) -> KeyLabels<'static> {
    match editor.mode() {
        Mode::View(_) => VIEW_KEYS,
        // No F-key acts in the form or the exit question.
        Mode::Form(_) | Mode::ConfirmExit(_) => LIST_KEYS.map(|(key, _)| (key, "")),
        Mode::Browse => list_key_labels(editor),
    }
}

/// The list's key bar: Del's label follows the selected server, and a
/// read-only dialog blanks the keys that change the table.
fn list_key_labels(editor: &LspEditor) -> KeyLabels<'static> {
    let mut labels = LIST_KEYS;
    if list::removes(editor) {
        relabel(&mut labels, "8", "Remove");
    }
    if matches!(editor.access(), Access::ReadOnly(_)) {
        for key in ["2", "4", "7", "8"] {
            relabel(&mut labels, key, "");
        }
    }
    labels
}

fn relabel(labels: &mut KeyLabels<'static>, key: &str, label: &'static str) {
    if let Some(cell) = labels.iter_mut().find(|(number, _)| *number == key) {
        cell.1 = label;
    }
}
