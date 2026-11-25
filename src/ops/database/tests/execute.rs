//! Purpose: Pin one request run on its own connection: SQLite end to end, and the
//! failures a connection can meet without a server.

use super::super::fixtures::{closed_port, column, items_page, name, shop, sqlite_spec, table_ref};
use super::super::{execute, DbError};
use crate::models::database::{
    ConnectionSpec, Driver, KeyRole, Nullability, Reply, Schema, Table, TableKind, Task,
};

const PASSWORD_VARIABLE: &str = "FFM_TEST_DATABASE_PASSWORD_THAT_IS_NEVER_SET";

fn rows_task(spec: &ConnectionSpec, offset: usize) -> Task {
    Task::Rows {
        spec: spec.clone(),
        table: table_ref("main", "items"),
        offset,
    }
}

#[test]
fn schemas_come_with_their_tables_and_views() {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let Ok(Reply::Schemas(schemas)) = execute(&Task::Schemas(shop(dir.path()))) else {
        panic!("the schemas")
    };
    let table = |text: &str, kind| Table {
        name: name(text),
        kind,
    };
    assert_eq!(
        schemas,
        vec![Schema {
            name: name("main"),
            tables: vec![
                table("items", TableKind::Table),
                table("labels", TableKind::View)
            ],
        }]
    );
}

#[test]
fn columns_come_with_their_type_key_and_nullability() {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let task = Task::Columns {
        spec: shop(dir.path()),
        table: table_ref("main", "items"),
    };
    let Ok(Reply::Columns(columns)) = execute(&task) else {
        panic!("the columns")
    };
    assert_eq!(
        columns,
        vec![
            column("id", "INTEGER", KeyRole::Primary, Nullability::Nullable),
            column("label", "TEXT", KeyRole::Plain, Nullability::NotNull),
            column("note", "TEXT", KeyRole::Plain, Nullability::Nullable),
        ]
    );
}

#[test]
fn rows_start_at_the_offset_and_show_a_null_as_null() {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let spec = shop(dir.path());
    let Ok(Reply::Rows(all)) = execute(&rows_task(&spec, 0)) else {
        panic!("the rows")
    };
    assert_eq!(
        all,
        items_page(&[&["1", "apple", "NULL"], &["2", "pear", "ripe"]])
    );
    let Ok(Reply::Rows(rest)) = execute(&rows_task(&spec, 1)) else {
        panic!("the rows from the second")
    };
    assert_eq!(rest, items_page(&[&["2", "pear", "ripe"]]));
    let Ok(Reply::Rows(none)) = execute(&rows_task(&spec, 9)) else {
        panic!("the rows past the end")
    };
    assert_eq!(none, items_page(&[]));
}

#[test]
fn every_request_on_a_missing_file_fails_with_the_drivers_text() {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let spec = sqlite_spec(dir.path().join("absent.db"));
    let tasks = [
        Task::Schemas(spec.clone()),
        Task::Columns {
            spec: spec.clone(),
            table: table_ref("main", "items"),
        },
        rows_task(&spec, 0),
    ];
    for task in tasks {
        let Err(DbError::Driver(text)) = execute(&task) else {
            panic!("a driver error")
        };
        assert!(text.contains("unable to open database file"), "{text}");
    }
}

fn postgres_spec(password_env: Option<&str>) -> ConnectionSpec {
    let mut endpoint = closed_port();
    endpoint.password_env = password_env.map(str::to_owned);
    ConnectionSpec {
        name: name("warehouse"),
        driver: Driver::Postgres(endpoint),
    }
}

#[test]
fn a_password_variable_that_is_unset_fails_before_any_connection() {
    let spec = postgres_spec(Some(PASSWORD_VARIABLE));
    let Err(error) = execute(&Task::Schemas(spec)) else {
        panic!("a missing password")
    };
    assert_eq!(
        error,
        DbError::MissingPassword(PASSWORD_VARIABLE.to_owned())
    );
    assert_eq!(
        error.to_string(),
        format!("environment variable {PASSWORD_VARIABLE} is not set")
    );
}

#[test]
fn a_postgres_entry_on_a_closed_port_fails_with_the_drivers_text() {
    let Err(DbError::Driver(text)) = execute(&Task::Schemas(postgres_spec(None))) else {
        panic!("a driver error")
    };
    assert!(text.contains("connecting to server"), "{text}");
}
