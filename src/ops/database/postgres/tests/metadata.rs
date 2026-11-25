//! Purpose: Pin the metadata the PostgreSQL driver reads from a test server's
//! catalogs: user schemas only, tables and views with their kind, and columns with
//! their type, primary key and nullability. Each test works in a schema of its
//! own and drops it. Runs only when `FFM_TEST_POSTGRES_URL` names a test server.

use super::super::{columns, schemas, tables};
use crate::models::database::{KeyRole, Nullability, TableKind};
use crate::ops::database::fixtures::{column, name, table_ref};
use crate::ops::database::test_server::{server, Scratch};

#[test]
fn schemas_are_the_user_schemas_and_never_a_system_one() {
    let Some(server) = server() else { return };
    let scratch = Scratch::create(&server, "schemas");
    let found = schemas(&mut server.connect()).expect("schemas");
    assert!(found.iter().any(|schema| schema.as_str() == scratch.schema));
    assert!(found.iter().all(|schema| {
        !schema.as_str().starts_with("pg_") && schema.as_str() != "information_schema"
    }));
}

#[test]
fn tables_and_views_come_with_their_kind() {
    let Some(server) = server() else { return };
    let mut scratch = Scratch::create(&server, "tables");
    scratch.run(
        "CREATE TABLE @schema.items (id integer);
         CREATE VIEW @schema.item_ids AS SELECT id FROM @schema.items;
         CREATE TABLE @schema.\"odd\"\"name.v 1\" (a integer);",
    );
    let mut client = server.connect();
    let mut listed: Vec<(String, TableKind)> = tables(&mut client, &name(&scratch.schema))
        .expect("tables")
        .into_iter()
        .map(|table| (table.name.to_string(), table.kind))
        .collect();
    listed.sort_by(|left, right| left.0.cmp(&right.0));
    assert_eq!(
        listed,
        vec![
            ("item_ids".to_owned(), TableKind::View),
            ("items".to_owned(), TableKind::Table),
            ("odd\"name.v 1".to_owned(), TableKind::Table),
        ]
    );
}

#[test]
fn a_schema_without_tables_lists_none() {
    let Some(server) = server() else { return };
    let scratch = Scratch::create(&server, "empty");
    let found = tables(&mut server.connect(), &name(&scratch.schema)).expect("tables");
    assert_eq!(found, Vec::new());
}

#[test]
fn columns_carry_their_type_primary_key_and_nullability() {
    let Some(server) = server() else { return };
    let mut scratch = Scratch::create(&server, "columns");
    scratch.run(
        "CREATE TABLE @schema.items (id integer PRIMARY KEY, label text NOT NULL, note text);",
    );
    let found =
        columns(&mut server.connect(), &table_ref(&scratch.schema, "items")).expect("columns");
    assert_eq!(
        found,
        vec![
            column("id", "integer", KeyRole::Primary, Nullability::NotNull),
            column("label", "text", KeyRole::Plain, Nullability::NotNull),
            column("note", "text", KeyRole::Plain, Nullability::Nullable),
        ]
    );
}

#[test]
fn every_part_of_a_composite_key_is_primary() {
    let Some(server) = server() else { return };
    let mut scratch = Scratch::create(&server, "composite");
    scratch
        .run("CREATE TABLE @schema.pairs (a integer, b integer, c integer, PRIMARY KEY (a, b));");
    let found =
        columns(&mut server.connect(), &table_ref(&scratch.schema, "pairs")).expect("columns");
    let keys: Vec<KeyRole> = found.iter().map(|column| column.key).collect();
    assert_eq!(keys, [KeyRole::Primary, KeyRole::Primary, KeyRole::Plain]);
}

#[test]
fn a_key_of_a_same_named_table_in_another_schema_is_not_taken() {
    let Some(server) = server() else { return };
    let mut keyed = Scratch::create(&server, "keyed");
    let mut plain = Scratch::create(&server, "plain");
    keyed.run("CREATE TABLE @schema.items (id integer PRIMARY KEY);");
    plain.run("CREATE TABLE @schema.items (id integer);");
    let found =
        columns(&mut server.connect(), &table_ref(&plain.schema, "items")).expect("columns");
    assert_eq!(found[0].key, KeyRole::Plain);
}

#[test]
fn a_view_and_an_odd_name_read_their_columns_and_a_missing_table_has_none() {
    let Some(server) = server() else { return };
    let mut scratch = Scratch::create(&server, "odd");
    scratch.run(
        "CREATE TABLE @schema.\"odd\"\"name.v 1\" (\"a \"\"b\"\"\" text NOT NULL);
         CREATE VIEW @schema.shown AS SELECT \"a \"\"b\"\"\" AS shown FROM @schema.\"odd\"\"name.v 1\";",
    );
    let mut client = server.connect();
    let odd = columns(&mut client, &table_ref(&scratch.schema, "odd\"name.v 1")).expect("columns");
    assert_eq!(
        odd,
        vec![column(
            "a \"b\"",
            "text",
            KeyRole::Plain,
            Nullability::NotNull
        )]
    );
    let view = columns(&mut client, &table_ref(&scratch.schema, "shown")).expect("columns");
    assert_eq!(view.len(), 1);
    assert_eq!(view[0].name, "shown");
    let missing = columns(&mut client, &table_ref(&scratch.schema, "absent")).expect("columns");
    assert_eq!(missing, Vec::new());
}
