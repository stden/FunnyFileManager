//! Purpose: Driver error paths for corrupt files and missing schemas or tables.

use super::{columns, connect, name, rows, shop, table_ref, tables};
use crate::ops::database::DbError;

#[test]
fn non_database_files_and_missing_relations_report_driver_errors() {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let invalid = dir.path().join("invalid.db");
    std::fs::write(&invalid, b"not a sqlite database").expect("write");

    assert!(matches!(
        connect(&dir.path().join("missing.db")),
        Err(DbError::Driver(_))
    ));

    let uninitialized = rusqlite::Connection::open(&invalid).expect("lazy open");
    assert!(matches!(
        columns(&uninitialized, &table_ref("main", "items")),
        Err(DbError::Driver(_))
    ));

    let valid = connect(&shop(&dir)).expect("open");
    assert!(matches!(
        tables(&valid, &name("missing_schema")),
        Err(DbError::Driver(_))
    ));
    assert!(matches!(
        rows(&valid, &table_ref("main", "missing_table"), 0),
        Err(DbError::Driver(_))
    ));
}
