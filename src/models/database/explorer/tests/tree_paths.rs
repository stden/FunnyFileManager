//! Purpose: The tree's lookups by path: the connection a row belongs to, the
//! schema and table a path names, and the expansion a row opens or closes.

use super::support::ITEMS;
use super::*;

#[test]
fn a_path_finds_its_connection_schema_and_table() {
    let mut queue = Requests::default();
    let mut explorer = loaded(&mut queue);
    assert_eq!(tree::connection_of(items_row()), 0);
    assert!(tree::schema_at(&explorer.connections, 0, 0).is_some());
    assert!(tree::schema_at(&explorer.connections, 0, 1).is_none());
    assert!(tree::table_at(&explorer.connections, ITEMS).is_some());

    let expansion = tree::expansion_mut(&mut explorer.connections, items_row());
    *expansion.expect("the table row has an expansion") = Expansion::Expanded;
    let items = tree::table_mut(&mut explorer.connections, ITEMS).expect("the items table");
    assert_eq!(items.expansion, Expansion::Expanded);
}

#[test]
fn a_new_schema_node_starts_collapsed_with_every_table() {
    let node = nodes::SchemaNode::from_schema(Schema {
        name: Name::new("main").expect("a name"),
        tables: vec![
            table("items", TableKind::Table),
            table("item_labels", TableKind::View),
        ],
    });
    assert_eq!(node.expansion, Expansion::Collapsed);
    assert_eq!(node.tables.len(), 2);
}

#[test]
fn a_row_knows_its_parent_up_to_the_connection() {
    let column = Row::Column(ITEMS, 0);
    assert_eq!(tree::parent(column), Some(items_row()));
    assert_eq!(tree::parent(items_row()), Some(Row::Schema(0, 0)));
    assert_eq!(tree::parent(Row::Schema(0, 0)), Some(Row::Connection(0)));
    assert_eq!(tree::parent(Row::Connection(0)), None);
    assert_eq!(tree::connection_of(column), 0);
    assert_eq!(tree::connection_of(Row::Schema(2, 0)), 2);
}

#[test]
fn a_row_is_open_only_while_it_is_expanded() {
    let mut queue = Requests::default();
    let mut explorer = loaded(&mut queue);
    let schema = Row::Schema(0, 0);
    assert!(tree::is_open(&explorer.connections, Row::Connection(0)));
    assert!(!tree::is_open(&explorer.connections, schema));
    assert!(!tree::is_open(&explorer.connections, items_row()));

    explorer.move_down();
    explorer.expand(&mut queue);
    move_to(&mut explorer, items_row());
    explorer.expand(&mut queue);
    assert!(tree::is_open(&explorer.connections, schema));
    assert!(tree::is_open(&explorer.connections, items_row()));
    assert!(!tree::is_open(&explorer.connections, Row::Column(ITEMS, 0)));
    assert!(!tree::is_open(&explorer.connections, Row::Connection(7)));
    assert!(!tree::is_open(&explorer.connections, Row::Schema(0, 7)));
    assert!(!tree::is_open(
        &explorer.connections,
        Row::Table(TablePath { table: 7, ..ITEMS })
    ));
}

#[test]
fn a_column_has_no_expansion_to_open() {
    let mut queue = Requests::default();
    let mut explorer = loaded(&mut queue);
    assert!(tree::expansion_mut(&mut explorer.connections, Row::Column(ITEMS, 0)).is_none());
}

#[test]
fn nothing_under_an_unloaded_connection_is_found() {
    let mut explorer = DatabaseExplorer::new(vec![spec("local")], None);
    assert!(tree::schema_at(&explorer.connections, 0, 0).is_none());
    assert!(tree::table_at(&explorer.connections, ITEMS).is_none());
    assert!(tree::table_mut(&mut explorer.connections, ITEMS).is_none());
    assert!(tree::expansion_mut(&mut explorer.connections, items_row()).is_none());
}

#[test]
fn an_open_table_shows_its_columns_beneath_it() {
    let mut queue = Requests::default();
    let mut explorer = loaded(&mut queue);
    explorer.move_down();
    explorer.expand(&mut queue);
    move_to(&mut explorer, items_row());
    explorer.expand(&mut queue);
    assert_eq!(explorer.rows().len(), 4, "no columns until they are loaded");
    answer(&mut explorer, &mut queue, columns());
    assert_eq!(
        explorer.rows(),
        [
            Row::Connection(0),
            Row::Schema(0, 0),
            items_row(),
            Row::Column(ITEMS, 0),
            Row::Table(TablePath { table: 1, ..ITEMS }),
        ]
    );

    explorer.collapse();
    assert_eq!(explorer.rows().len(), 4, "a closed table hides its columns");
}
