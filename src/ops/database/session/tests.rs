//! Purpose: Pin what a session decides itself: how it opens (the password is read
//! before any connection), where a page ends, and how a cell is made printable.

use super::{page_of, printable, Session};
use crate::models::database::{ConnectionSpec, Driver, PageEnd, PAGE_ROWS};
use crate::ops::database::fixtures::{closed_port, name, sqlite_spec};
use crate::ops::database::DbError;

fn rows(count: usize) -> Vec<Vec<String>> {
    (0..count).map(|index| vec![index.to_string()]).collect()
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
fn a_page_of_exactly_the_page_size_is_the_last() {
    let page = page_of(vec!["n".to_owned()], rows(PAGE_ROWS));
    assert_eq!(page.rows.len(), PAGE_ROWS);
    assert_eq!(page.end, PageEnd::Last);
}

#[test]
fn the_extra_row_is_dropped_and_says_a_further_page_exists() {
    let page = page_of(vec!["n".to_owned()], rows(PAGE_ROWS + 1));
    assert_eq!(page.rows, rows(PAGE_ROWS));
    assert_eq!(page.columns, vec!["n".to_owned()]);
    assert_eq!(page.end, PageEnd::More);
}

#[test]
fn a_page_with_no_rows_is_the_last() {
    let page = page_of(vec!["n".to_owned()], Vec::new());
    assert_eq!(page.rows, Vec::<Vec<String>>::new());
    assert_eq!(page.end, PageEnd::Last);
}

#[test]
fn control_characters_show_as_dots_and_other_text_is_unchanged() {
    assert_eq!(printable("a\tb\nc\u{7}d\u{1b}"), "a·b·c·d·");
    assert_eq!(printable("plain ünï 日本"), "plain ünï 日本");
    assert_eq!(printable(""), "");
}

#[test]
fn a_missing_sqlite_file_does_not_open() {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let path = dir.path().join("absent.db");
    let Err(DbError::Driver(text)) = Session::open(&sqlite_spec(path.clone())) else {
        panic!("a driver error")
    };
    assert!(text.contains("unable to open database file"), "{text}");
    assert!(!path.exists());
}

#[test]
fn the_password_is_looked_up_before_a_postgres_connection_is_tried() {
    let variable = "FFM_TEST_SESSION_PASSWORD_THAT_IS_NEVER_SET";
    let Err(error) = Session::open(&postgres_spec(Some(variable))) else {
        panic!("a missing password")
    };
    assert_eq!(error, DbError::MissingPassword(variable.to_owned()));
}

#[test]
fn a_postgres_server_that_cannot_be_reached_is_a_driver_error() {
    let Err(DbError::Driver(text)) = Session::open(&postgres_spec(None)) else {
        panic!("a driver error")
    };
    assert!(text.contains("connecting to server"), "{text}");
}
