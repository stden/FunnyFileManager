//! Purpose: The tree's lookups by path: the connection a row belongs to, the
//! schema and table a path names, and the expansion a row opens or closes.

use super::*;

const ITEMS: TablePath = TablePath {
    connection: 0,
    schema: 0,
    table: 0,
};

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
