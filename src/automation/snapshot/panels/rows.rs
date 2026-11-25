//! Purpose: Bound file listings around their actual cursor and preserve global row indexes.

use crate::models::{DirEntry, Panel, RemotePanel};
use serde_json::{json, Value};

const LIMIT: usize = 500;

pub(super) fn listing<T>(
    rows: &[T],
    cursor: Option<usize>,
    mut row: impl FnMut(&T) -> Value,
) -> Value {
    let cursor = cursor.filter(|&index| index < rows.len());
    let offset = cursor
        .unwrap_or(0)
        .saturating_sub(LIMIT / 2)
        .min(rows.len().saturating_sub(LIMIT));
    let entries: Vec<_> = rows
        .iter()
        .enumerate()
        .skip(offset)
        .take(LIMIT)
        .map(|(index, entry)| {
            let mut value = row(entry);
            field(&mut value, "index", json!(index));
            field(&mut value, "selected", json!(cursor == Some(index)));
            value
        })
        .collect();
    json!({"entries":entries,"total_entries":rows.len(),"entry_offset":offset,
        "entries_truncated":rows.len()>LIMIT,"selected_index":cursor})
}

pub(super) fn local(panel: &Panel) -> Value {
    let mut value = listing(&panel.entries, panel.selected(), |entry| {
        file(entry, panel.marked.contains(entry.name.as_str()))
    });
    field(&mut value, "kind", json!("files"));
    field(&mut value, "path", json!(panel.path.to_string_lossy()));
    value
}

pub(super) fn remote(panel: &RemotePanel) -> Value {
    let mut value = listing(panel.entries(), Some(panel.cursor()), |entry| {
        file(entry, panel.marked(&entry.name))
    });
    field(&mut value, "kind", json!("remote"));
    field(&mut value, "path", json!(panel.path().as_str()));
    field(&mut value, "session", json!(panel.session().as_str()));
    value
}

fn file(entry: &DirEntry, marked: bool) -> Value {
    json!({"name":entry.name,"is_dir":entry.kind.is_dir(),
        "is_symlink":entry.kind.is_symlink(),"size":entry.size.bytes(),"marked":marked})
}

pub(super) fn field(value: &mut Value, name: &str, field: Value) {
    if let Some(object) = value.as_object_mut() {
        drop(object.insert(name.to_owned(), field));
    }
}
