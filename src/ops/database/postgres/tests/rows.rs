//! Purpose: Pin the rows the PostgreSQL driver reads from a test server: pages of
//! 200 with a shorter last page, every type shown as text, a NULL shown as NULL,
//! control characters shown as dots, and a table that is missing refused. Runs
//! only when `FFM_TEST_POSTGRES_URL` names a test server.

use super::super::rows;
use crate::models::database::PageEnd;
use crate::ops::database::fixtures::table_ref;
use crate::ops::database::test_server::{server, Scratch};
use crate::ops::database::DbError;

#[test]
fn rows_come_in_pages_of_200_with_a_shorter_last_page() {
    let Some(server) = server() else { return };
    let mut scratch = Scratch::create(&server, "paging");
    scratch.run(
        "CREATE TABLE @schema.big (n integer);
         INSERT INTO @schema.big SELECT generate_series(1, 450);",
    );
    let mut client = server.connect();
    let big = table_ref(&scratch.schema, "big");

    let first = rows(&mut client, &big, 0).expect("first page");
    assert_eq!(first.rows.len(), 200);
    assert_eq!(first.end, PageEnd::More);
    assert_eq!(first.rows[0][0], "1");
    assert_eq!(first.columns, vec!["n".to_owned()]);

    let second = rows(&mut client, &big, 200).expect("second page");
    assert_eq!(second.rows.len(), 200);
    assert_eq!(second.end, PageEnd::More);
    assert_eq!(second.rows[0][0], "201");

    let last = rows(&mut client, &big, 400).expect("last page");
    assert_eq!(last.rows.len(), 50);
    assert_eq!(last.end, PageEnd::Last);
    assert_eq!(last.rows[49][0], "450");
}

/// A table of several types: a row of values and a row of NULLs.
const KINDS_SQL: &str =
    "CREATE TABLE @schema.kinds (id integer, flag boolean, at timestamp, data bytea, note text);
     INSERT INTO @schema.kinds VALUES
       (1, true, '2024-01-02 03:04:05', '\\xdeadbeef', E'tab\\there\\nbreak'),
       (2, NULL, NULL, NULL, NULL);";

const VALUES_ROW: [&str; 5] = [
    "1",
    "true",
    "2024-01-02 03:04:05",
    "\\xdeadbeef",
    "tab·here·break",
];

#[test]
fn every_type_is_shown_as_text_and_a_null_as_null() {
    let Some(server) = server() else { return };
    let mut scratch = Scratch::create(&server, "values");
    scratch.run(KINDS_SQL);
    let kinds = table_ref(&scratch.schema, "kinds");
    let page = rows(&mut server.connect(), &kinds, 0).expect("rows");
    assert_eq!(
        page.columns,
        ["id", "flag", "at", "data", "note"].map(str::to_owned)
    );
    assert_eq!(
        page.rows,
        vec![
            VALUES_ROW.map(str::to_owned),
            ["2", "NULL", "NULL", "NULL", "NULL"].map(str::to_owned),
        ]
    );
    assert_eq!(page.end, PageEnd::Last);
}

#[test]
fn an_empty_table_and_an_odd_name_still_give_their_column_names() {
    let Some(server) = server() else { return };
    let mut scratch = Scratch::create(&server, "empty_rows");
    scratch.run("CREATE TABLE @schema.\"odd\"\"name.v 1\" (\"a \"\"b\"\"\" text);");
    let page = rows(
        &mut server.connect(),
        &table_ref(&scratch.schema, "odd\"name.v 1"),
        0,
    )
    .expect("rows");
    assert_eq!(page.columns, vec!["a \"b\"".to_owned()]);
    assert_eq!(page.rows, Vec::<Vec<String>>::new());
    assert_eq!(page.end, PageEnd::Last);
}

#[test]
fn a_table_that_is_missing_is_a_driver_error() {
    let Some(server) = server() else { return };
    let scratch = Scratch::create(&server, "missing");
    let Err(DbError::Driver(text)) = rows(
        &mut server.connect(),
        &table_ref(&scratch.schema, "absent"),
        0,
    ) else {
        panic!("a driver error")
    };
    assert!(text.starts_with("db error"), "{text}");
}
