//! Purpose: The columns view: one table's columns with their type, primary key
//! and nullability, once they are loaded; the state until then.

use ratatui::text::Line;

use crate::models::database::{Column, DatabaseExplorer, KeyRole, Load, Nullability, TablePath};

/// Only opened once its columns are queued, so `Unloaded` means loading.
pub(super) fn lines(
    explorer: &DatabaseExplorer,
    path: TablePath,
    scroll: usize,
) -> Vec<Line<'static>> {
    let title = explorer
        .table(path)
        .map_or_else(String::new, |table| format!("Columns of {}", table.name));
    let mut lines = vec![Line::from(title), Line::from(header())];
    let body = match explorer.table(path).map(|table| &table.columns) {
        Some(Load::Ready(columns)) => columns.iter().map(describe).collect(),
        Some(Load::Failed(message)) => vec![message.clone()],
        Some(Load::Unloaded) | None => vec!["Loading…".to_owned()],
    };
    lines.extend(body.into_iter().skip(scroll).map(Line::from));
    lines
}

fn header() -> String {
    format!(
        "{:<24} {:<20} {:<12} {}",
        "Name", "Type", "Primary key", "Nullable"
    )
}

fn describe(column: &Column) -> String {
    let key = match column.key {
        KeyRole::Primary => "yes",
        KeyRole::Plain => "",
    };
    let nullable = match column.nullability {
        Nullability::Nullable => "yes",
        Nullability::NotNull => "no",
    };
    format!(
        "{:<24} {:<20} {:<12} {}",
        column.name, column.data_type, key, nullable
    )
}
