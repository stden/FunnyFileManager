//! Purpose: The PostgreSQL driver against a real server, run only when
//! `FFM_TEST_POSTGRES_URL` names one. Without it the test returns at once; the
//! query builders are pinned without a server in `sql/tests.rs`.

use super::{read_only, schemas, tables};

mod connect;
mod metadata;
mod rows;

#[test]
fn a_test_server_answers_the_metadata_queries_when_its_url_is_set() {
    let url = std::env::var("FFM_TEST_POSTGRES_URL")
        .unwrap_or_else(|_| crate::ops::database::tests::pg_wire::url());
    let mut client =
        postgres::Client::connect(&url, postgres::NoTls).expect("connect to the test server");
    read_only(&mut client).expect("a read-only session");
    let found = schemas(&mut client).expect("schemas");
    assert!(found
        .iter()
        .all(|schema| !schema.as_str().starts_with("pg_")));
    if let Some(first) = found.first() {
        tables(&mut client, first).expect("tables");
    }
}
