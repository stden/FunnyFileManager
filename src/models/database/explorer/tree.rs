//! Purpose: The metadata tree behind the explorer: the connections and what each
//! holds, the rows the tree shows top to bottom, and the parent of each row.

pub(crate) use super::nodes::{Connection, Expansion, Load, Row, SchemaNode, TableNode, TablePath};

/// The rows the tree shows: a connection's schemas while it is open, a
/// schema's tables while it is open, a table's columns while it is open.
pub(crate) fn visible(connections: &[Connection]) -> Vec<Row> {
    let mut rows = Vec::new();
    for (connection_index, connection) in connections.iter().enumerate() {
        rows.push(Row::Connection(connection_index));
        if let (Expansion::Expanded, Some(schemas)) =
            (connection.expansion, connection.schemas.ready())
        {
            push_schemas(&mut rows, connection_index, schemas);
        }
    }
    rows
}

fn push_schemas(rows: &mut Vec<Row>, connection: usize, schemas: &[SchemaNode]) {
    for (schema_index, schema) in schemas.iter().enumerate() {
        rows.push(Row::Schema(connection, schema_index));
        if schema.expansion == Expansion::Expanded {
            push_tables(rows, connection, schema_index, &schema.tables);
        }
    }
}

fn push_tables(rows: &mut Vec<Row>, connection: usize, schema: usize, tables: &[TableNode]) {
    for (table_index, table) in tables.iter().enumerate() {
        let path = TablePath {
            connection,
            schema,
            table: table_index,
        };
        rows.push(Row::Table(path));
        if let (Expansion::Expanded, Some(columns)) = (table.expansion, table.columns.ready()) {
            rows.extend((0..columns.len()).map(|column| Row::Column(path, column)));
        }
    }
}

pub(crate) const fn parent(row: Row) -> Option<Row> {
    match row {
        Row::Connection(_) => None,
        Row::Schema(connection, _) => Some(Row::Connection(connection)),
        Row::Table(path) => Some(Row::Schema(path.connection, path.schema)),
        Row::Column(path, _) => Some(Row::Table(path)),
    }
}

/// The connection a row belongs to.
pub(crate) const fn connection_of(row: Row) -> usize {
    match row {
        Row::Connection(connection) | Row::Schema(connection, _) => connection,
        Row::Table(path) | Row::Column(path, _) => path.connection,
    }
}

pub(crate) fn is_open(connections: &[Connection], row: Row) -> bool {
    match row {
        Row::Connection(connection) => connections
            .get(connection)
            .is_some_and(|node| node.expansion == Expansion::Expanded),
        Row::Schema(connection, schema) => schema_at(connections, connection, schema)
            .is_some_and(|node| node.expansion == Expansion::Expanded),
        Row::Table(path) => {
            table_at(connections, path).is_some_and(|node| node.expansion == Expansion::Expanded)
        }
        Row::Column(..) => false,
    }
}

pub(crate) fn schema_at(
    connections: &[Connection],
    connection: usize,
    schema: usize,
) -> Option<&SchemaNode> {
    connections.get(connection)?.schemas.ready()?.get(schema)
}

pub(crate) fn table_at(connections: &[Connection], path: TablePath) -> Option<&TableNode> {
    schema_at(connections, path.connection, path.schema)?
        .tables
        .get(path.table)
}

pub(crate) fn expansion_mut(connections: &mut [Connection], row: Row) -> Option<&mut Expansion> {
    match row {
        Row::Connection(connection) => connections
            .get_mut(connection)
            .map(|node| &mut node.expansion),
        Row::Schema(connection, schema) => {
            schema_mut(connections, connection, schema).map(|node| &mut node.expansion)
        }
        Row::Table(path) => table_mut(connections, path).map(|node| &mut node.expansion),
        Row::Column(..) => None,
    }
}

pub(crate) fn table_mut(connections: &mut [Connection], path: TablePath) -> Option<&mut TableNode> {
    schema_mut(connections, path.connection, path.schema)?
        .tables
        .get_mut(path.table)
}

fn schema_mut(
    connections: &mut [Connection],
    connection: usize,
    schema: usize,
) -> Option<&mut SchemaNode> {
    let Load::Ready(schemas) = &mut connections.get_mut(connection)?.schemas else {
        return None;
    };
    schemas.get_mut(schema)
}
