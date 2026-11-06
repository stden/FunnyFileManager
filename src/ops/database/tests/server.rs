//! Purpose: Pin the three requests run against the PostgreSQL test server, when
//! `FFM_TEST_POSTGRES_URL` names one: the answers a configured connection gives.

use super::super::execute;
use super::super::fixtures::{column, items_page, table_ref};
use super::super::test_server::{server, Scratch};
use crate::models::database::{
    ConnectionSpec, Driver, KeyRole, Nullability, Reply, TableRef, Task,
};

fn assert_server_schemas(spec: &ConnectionSpec, scratch: &str) {
    let Ok(Reply::Schemas(schemas)) = execute(&Task::Schemas(spec.clone())) else {
        panic!("the schemas")
    };
    let schema = schemas
        .iter()
        .find(|schema| schema.name.as_str() == scratch)
        .expect("the scratch schema is listed");
    assert_eq!(schema.tables.len(), 1);
    assert_eq!(schema.tables[0].name.as_str(), "items");
}

fn assert_server_columns(spec: &ConnectionSpec, table: &TableRef) {
    let task = Task::Columns {
        spec: spec.clone(),
        table: table.clone(),
    };
    let Ok(Reply::Columns(columns)) = execute(&task) else {
        panic!("the columns")
    };
    assert_eq!(
        columns,
        vec![
            column("id", "integer", KeyRole::Primary, Nullability::NotNull),
            column("label", "text", KeyRole::Plain, Nullability::NotNull),
            column("note", "text", KeyRole::Plain, Nullability::Nullable),
        ]
    );
}

fn assert_server_rows(spec: ConnectionSpec, table: TableRef) {
    let task = Task::Rows {
        spec,
        table,
        offset: 1,
    };
    let Ok(Reply::Rows(rows)) = execute(&task) else {
        panic!("the rows")
    };
    assert_eq!(rows, items_page(&[&["2", "pear", "ripe"]]));
}

#[test]
fn the_test_server_answers_the_three_requests() {
    let Some(server) = server() else { return };
    let mut scratch = Scratch::create(&server, "execute");
    scratch.run(
        "CREATE TABLE @schema.items (id integer PRIMARY KEY, label text NOT NULL, note text);
         INSERT INTO @schema.items VALUES (1, 'apple', NULL), (2, 'pear', 'ripe');",
    );
    let (spec, _password) = server.spec();
    let table = table_ref(&scratch.schema, "items");
    assert_server_schemas(&spec, &scratch.schema);
    assert_server_columns(&spec, &table);
    assert_server_rows(spec, table);
}

#[test]
fn a_url_password_reaches_the_driver_through_its_variable() {
    use super::super::test_server::from_url;
    let server = from_url("postgres://ffm:secret@127.0.0.1:5432/ffm".to_owned());
    let (spec, _variable) = server.spec();
    let Driver::Postgres(endpoint) = spec.driver else {
        panic!("a PostgreSQL connection")
    };
    assert_eq!(
        endpoint.password_env.as_deref(),
        Some("FFM_TEST_POSTGRES_PASSWORD")
    );
}

#[test]
#[should_panic(expected = "names a TCP host")]
fn a_socket_url_is_refused_by_the_test_server() {
    super::super::test_server::from_url(
        "postgres://postgres@/ffm?host=/var/run/postgresql".to_owned(),
    );
}
