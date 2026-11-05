//! Purpose: The rows view: one page of a table's rows, every value as text, with
//! the range the page covers; the state while it loads or when it failed.

use ratatui::text::Line;

use crate::models::database::{Load, Page, PageEnd, RowsView};

/// Only opened once its first page is queued, so `Unloaded` means loading.
pub(super) fn lines(rows: &RowsView) -> Vec<Line<'static>> {
    let title = format!(
        "{}.{} — {}",
        rows.table.schema,
        rows.table.name,
        range(rows)
    );
    let body = match &rows.page {
        Load::Ready(page) => table_lines(page),
        Load::Failed(message) => vec![message.clone()],
        Load::Unloaded => vec!["Loading…".to_owned()],
    };
    let mut lines = vec![Line::from(title)];
    lines.extend(body.into_iter().skip(rows.scroll).map(Line::from));
    lines
}

fn range(rows: &RowsView) -> String {
    match &rows.page {
        Load::Ready(page) if page.rows.is_empty() => "no rows".to_owned(),
        Load::Ready(page) => {
            let first = rows.offset + 1;
            let last = rows.offset + page.rows.len();
            let more = if page.end == PageEnd::More {
                ", more follow"
            } else {
                ""
            };
            format!("rows {first}–{last}{more}")
        }
        Load::Unloaded | Load::Failed(_) => format!("from row {}", rows.offset + 1),
    }
}

/// The header line of the page, then one line per row, cells separated by bars.
fn table_lines(page: &Page) -> Vec<String> {
    let mut lines = vec![page.columns.join(" │ ")];
    lines.extend(page.rows.iter().map(|cells| cells.join(" │ ")));
    lines
}
