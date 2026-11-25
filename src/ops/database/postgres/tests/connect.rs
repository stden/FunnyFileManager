//! Purpose: Pin how a session opens and fails: read-only with a statement time
//! limit, driver errors with the password taken out, and every query of a dead
//! connection reported as a driver error. The server cases run only when
//! `FFM_TEST_POSTGRES_URL` names a test server.

use super::super::{columns, connect, read_only, rows, schemas, tables};
use crate::ops::database::fixtures::{closed_port, name, table_ref};
use crate::ops::database::test_server::server;
use crate::ops::database::DbError;

const SECRET: &str = "hunter2";

#[test]
fn a_port_nothing_listens_on_is_a_driver_error_that_does_not_show_the_password() {
    let Err(DbError::Driver(text)) = connect(&closed_port(), Some(SECRET)) else {
        panic!("a driver error")
    };
    assert!(text.contains("connecting to server"), "{text}");
    assert!(!text.contains(SECRET), "{text}");
}

#[test]
fn a_session_is_read_only_and_cuts_a_long_statement_short() {
    let Some(server) = server() else { return };
    let mut client = server.connect();
    let read_only: String = client
        .query_one("SHOW transaction_read_only", &[])
        .expect("the setting")
        .get(0);
    assert_eq!(read_only, "on");
    let timeout: String = client
        .query_one("SHOW statement_timeout", &[])
        .expect("the setting")
        .get(0);
    assert_eq!(timeout, "5s");
    let refused = client
        .batch_execute("CREATE SCHEMA ffm_test_refused")
        .expect_err("a write is refused");
    let message = refused
        .as_db_error()
        .expect("the server's refusal")
        .message();
    assert!(message.contains("read-only transaction"), "{message}");
}

#[test]
fn a_refused_login_is_a_driver_error_that_does_not_show_the_password() {
    let Some(server) = server() else { return };
    let mut endpoint = server.endpoint;
    endpoint.user = SECRET.to_owned();
    let Err(DbError::Driver(text)) = connect(&endpoint, Some(SECRET)) else {
        panic!("a refused login")
    };
    assert!(text.starts_with("db error"), "{text}");
    assert!(!text.contains(SECRET), "{text}");
}

#[test]
fn every_query_on_a_dead_connection_is_a_driver_error() {
    let Some(server) = server() else { return };
    let mut dead = server.dead_client();
    let table = table_ref("public", "anything");
    assert!(matches!(read_only(&mut dead), Err(DbError::Driver(_))));
    assert!(matches!(schemas(&mut dead), Err(DbError::Driver(_))));
    assert!(matches!(
        tables(&mut dead, &name("public")),
        Err(DbError::Driver(_))
    ));
    assert!(matches!(
        columns(&mut dead, &table),
        Err(DbError::Driver(_))
    ));
    assert!(matches!(
        rows(&mut dead, &table, 0),
        Err(DbError::Driver(_))
    ));
}
