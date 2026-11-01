//! Purpose: Describe the quick-view source and position without inventing file-list rows.

use crate::models::{Preview, QuickView};
use serde_json::{json, Value};

pub(super) fn describe(view: &QuickView) -> Value {
    let preview = match &view.preview {
        Preview::Nothing => json!({"kind":"nothing"}),
        Preview::Text(text) => json!({"kind":"text","path":text.path.to_string_lossy(),
            "line_count":text.lines.len(),"size":text.size.bytes()}),
        Preview::Folder(_) => json!({"kind":"folder"}),
        Preview::Refused(reason) => json!({"kind":"refused","reason":reason.label()}),
        Preview::Image(state) => super::super::image::describe(state),
    };
    json!({"kind":"quick_view","path":view.shown.as_ref().map(|path|path.to_string_lossy()),
        "entries":[],"total_entries":0,"entries_truncated":false,"entry_offset":0,
        "selected_index":null,"preview":preview,"scroll":view.scroll.lines(),"column":view.col.cells()})
}
