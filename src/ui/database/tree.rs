//! Purpose: The lines of the explorer's tree: one per visible row, indented by
//! depth, marked open or closed, with the state of each node (loading, failed)
//! and the cursor's row reversed.

use ratatui::style::Style;
use ratatui::text::Line;

use crate::models::database::{
    DatabaseExplorer, KeyRole, Load, Nullability, Row, TableKind, TablePath,
};

pub(super) fn lines(explorer: &DatabaseExplorer) -> Vec<Line<'static>> {
    let cursor = explorer.cursor();
    explorer
        .rows()
        .into_iter()
        .enumerate()
        .map(|(index, row)| {
            let line = Line::from(format!("{}{}", indent(row), text(explorer, row)));
            if index == cursor {
                line.patch_style(Style::new().reversed())
            } else {
                line
            }
        })
        .collect()
}

fn indent(row: Row) -> &'static str {
    match row {
        Row::Connection(_) => "",
        Row::Schema(..) => "  ",
        Row::Table(_) => "    ",
        Row::Column(..) => "      ",
    }
}

fn text(explorer: &DatabaseExplorer, row: Row) -> String {
    let marker = match row {
        Row::Column(..) => "",
        _ if explorer.is_open(row) => "▾ ",
        _ => "▸ ",
    };
    let name = explorer.name(row).unwrap_or("");
    let detail = match row {
        Row::Connection(connection) => connection_detail(explorer, connection),
        Row::Schema(..) => String::new(),
        Row::Table(path) => table_detail(explorer, path),
        Row::Column(path, index) => column_detail(explorer, path, index),
    };
    let state = if explorer.is_loading_row(row) {
        " Loading…"
    } else {
        ""
    };
    format!("{marker}{name}{detail}{state}")
}

fn connection_detail(explorer: &DatabaseExplorer, connection: usize) -> String {
    let Some(node) = explorer.connections().get(connection) else {
        return String::new();
    };
    let failure = match &node.schemas {
        Load::Failed(message) => format!(" — {message}"),
        Load::Unloaded | Load::Ready(_) => String::new(),
    };
    format!(" ({}){failure}", node.spec.driver.kind_label())
}

fn table_detail(explorer: &DatabaseExplorer, path: TablePath) -> String {
    match explorer.table(path).map(|table| table.kind) {
        Some(TableKind::View) => " [view]".to_owned(),
        Some(TableKind::Table) | None => String::new(),
    }
}

fn column_detail(explorer: &DatabaseExplorer, path: TablePath, index: usize) -> String {
    let Some(column) = explorer
        .table(path)
        .and_then(|table| table.columns.ready())
        .and_then(|columns| columns.get(index))
    else {
        return String::new();
    };
    let key = match column.key {
        KeyRole::Primary => " PK",
        KeyRole::Plain => "",
    };
    let nullable = match column.nullability {
        Nullability::Nullable => "",
        Nullability::NotNull => " NOT NULL",
    };
    format!(" {}{key}{nullable}", column.data_type)
}

#[cfg(test)]
mod tests;
