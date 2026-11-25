//! Purpose: Pin the query builders of both drivers without a server: quoted
//! identifiers only, one extra row to learn about the next page, and every
//! metadata query taking its names as parameters.

use super::{select_rows, sqlite_columns, sqlite_tables, text_columns};
use super::{POSTGRES_COLUMNS, POSTGRES_TABLES};
use crate::models::database::{Name, TableRef};

fn name(text: &str) -> Name {
    Name::new(text).expect("a name")
}

fn table(schema: &str, table: &str) -> TableRef {
    TableRef {
        schema: name(schema),
        name: name(table),
    }
}

#[test]
fn sqlite_tables_read_the_quoted_schema_and_skip_internal_tables() {
    assert_eq!(
        sqlite_tables(&name("my schema")),
        "SELECT name, type FROM \"my schema\".sqlite_master WHERE type IN ('table', 'view') \
         AND substr(name, 1, 7) <> 'sqlite_' ORDER BY name"
    );
}

#[test]
fn sqlite_columns_ask_the_pragma_with_quoted_names() {
    assert_eq!(
        sqlite_columns(&table("main", "a \"b\".c")),
        "PRAGMA \"main\".table_info(\"a \"\"b\"\".c\")"
    );
}

#[test]
fn a_rows_query_asks_one_row_more_than_a_page_from_the_offset() {
    assert_eq!(
        select_rows(&table("main", "t"), "*", 400),
        "SELECT * FROM \"main\".\"t\" LIMIT 201 OFFSET 400"
    );
}

#[test]
fn postgres_values_are_read_as_text_in_column_order() {
    let names = ["id".to_owned(), "a b".to_owned()];
    assert_eq!(text_columns(&names), "\"id\"::text, \"a b\"::text");
    assert_eq!(
        select_rows(&table("public", "t"), &text_columns(&names), 0),
        "SELECT \"id\"::text, \"a b\"::text FROM \"public\".\"t\" LIMIT 201 OFFSET 0"
    );
}

#[test]
fn every_builder_escapes_quotes_and_keeps_dots_inside_one_identifier() {
    let odd = table("sch.ema", "odd\"name.v1");
    assert!(select_rows(&odd, "*", 0).contains("\"sch.ema\".\"odd\"\"name.v1\""));
    assert!(sqlite_columns(&odd).contains("\"odd\"\"name.v1\""));
}

#[test]
fn postgres_metadata_queries_take_parameters_and_never_a_name() {
    assert!(POSTGRES_TABLES.contains("$1"));
    assert!(!POSTGRES_TABLES.contains("public"));
    assert!(POSTGRES_COLUMNS.contains("$1") && POSTGRES_COLUMNS.contains("$2"));
}
