//! Purpose: Draws the record form: the zone and the record, the fields with the one under the
//! cursor, the note that says why a value is empty, the status line and the keys.

use super::lines::status;
use crate::models::cloudflare_dns::{CloudflareDnsPanel, FormField, Proxy, RecordForm, Target};
use crate::ui::dialog::{self, Ground, Row};
use ratatui::Frame;

const NEW_TITLE: &str = "New record";
const EDIT_TITLE: &str = "Edit record";
const KEYS: &str = "Tab next field | Space proxy | Enter review | Esc back";

pub(super) fn render(
    f: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    form: &RecordForm,
    panel: &CloudflareDnsPanel,
) {
    let title = match form.target {
        Target::New => NEW_TITLE,
        Target::Existing(_) => EDIT_TITLE,
    };
    let zone = format!("Zone: {}", form.zone.name);
    let record = match &form.target {
        Target::New => String::new(),
        Target::Existing(record) => format!("Record: {} ({})", record.name, record.kind),
    };
    let status = status(panel);
    let mut rows = vec![Row::Plain(&zone)];
    if matches!(form.target, Target::Existing(_)) {
        rows.push(Row::Plain(&record));
    }
    rows.extend(fields(form));
    if !form.note.is_empty() {
        rows.extend([Row::Divider, Row::Plain(&form.note)]);
    }
    rows.extend([Row::Divider, Row::Plain(&status), Row::Plain(KEYS)]);
    if let Some(caret) = dialog::render_tracked(f, tracker, title, Ground::Prompt, &rows, &[]) {
        f.set_cursor_position(caret);
    }
}

/// The editable rows: the name for a new record, then the content, the TTL and the proxy.
fn fields(form: &RecordForm) -> Vec<Row<'_>> {
    let focused = |field: FormField| form.field == field;
    let mut rows = Vec::new();
    if matches!(form.target, Target::New) {
        rows.push(Row::text_field(
            "Name",
            &form.name,
            focused(FormField::Name),
            true,
        ));
    }
    rows.extend([
        Row::text_field("Content", &form.content, focused(FormField::Content), true),
        Row::text_field(
            "TTL (1 = automatic)",
            &form.ttl,
            focused(FormField::Ttl),
            true,
        ),
    ]);
    rows.push(match form.proxy {
        Proxy::Unsupported => Row::Plain("Proxied: not available for this record type"),
        proxy => Row::checkbox("Proxied", proxy == Proxy::On, focused(FormField::Proxy)),
    });
    rows
}
