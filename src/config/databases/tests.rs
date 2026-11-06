//! Purpose: Pin `databases.yaml`: valid entries and their defaults, a missing,
//! unreadable or empty file, malformed YAML, and refused entries whose error
//! never repeats a value (a password above all).

use std::path::Path;

use super::{load, parse, LoadError, MAX_BYTES};
use crate::models::database::Driver;

const VALID: &str = "\
- name: scratch
  kind: sqlite
  path: /data/scratch.db
- name: warehouse
  kind: postgres
  database: analytics
  user: reader
  password_env: WAREHOUSE_PASSWORD
";

#[test]
fn valid_entries_parse_with_their_defaults() {
    let specs = parse(VALID).expect("valid entries");
    assert_eq!(specs.len(), 2);
    assert_eq!(specs[0].name.as_str(), "scratch");
    assert!(matches!(
        &specs[0].driver,
        Driver::Sqlite { path } if path == Path::new("/data/scratch.db")
    ));
    let Driver::Postgres(endpoint) = &specs[1].driver else {
        panic!("a postgres entry")
    };
    assert_eq!(endpoint.host, "localhost");
    assert_eq!(endpoint.port, 5432);
    assert_eq!(endpoint.database, "analytics");
    assert_eq!(endpoint.user, "reader");
    assert_eq!(endpoint.password_env.as_deref(), Some("WAREHOUSE_PASSWORD"));
}

#[test]
fn an_empty_file_lists_no_connections() {
    assert_eq!(parse("").expect("empty is valid"), Vec::new());
}

#[test]
fn a_missing_file_is_reported_as_missing() {
    let dir = tempfile::TempDir::new().expect("temp dir");
    assert_eq!(
        load(&dir.path().join("databases.yaml")),
        Err(LoadError::Missing)
    );
}

#[test]
fn a_directory_in_place_of_the_file_is_unreadable() {
    let dir = tempfile::TempDir::new().expect("temp dir");
    assert_eq!(load(dir.path()), Err(LoadError::Unreadable));
}

#[test]
fn malformed_yaml_reports_a_line_without_panicking() {
    let error = parse("- name: a\n  kind: [unclosed\n").expect_err("malformed");
    assert!(matches!(error, LoadError::Malformed { .. }));
}

#[test]
fn a_password_field_is_refused_and_never_echoed() {
    let text =
        "- name: pg\n  kind: postgres\n  database: a\n  user: b\n  password: hunter2-secret\n";
    let error = parse(text).expect_err("passwords are not allowed in the file");
    assert!(matches!(error, LoadError::Malformed { .. }));
    assert!(!error.to_string().contains("hunter2"));
}

#[test]
fn a_wrong_type_is_refused_without_echoing_the_value() {
    let text = "- name: pg\n  kind: postgres\n  port: hunter2-secret\n";
    let error = parse(text).expect_err("a port is a number");
    assert!(!error.to_string().contains("hunter2"));
}

#[test]
fn an_unknown_kind_is_refused() {
    let error = parse("- name: m\n  kind: mysql\n").expect_err("mysql is out of scope");
    assert!(matches!(error, LoadError::Malformed { .. }));
}

#[test]
fn a_postgres_entry_needs_a_database_and_a_user() {
    let error = parse("- name: pg\n  kind: postgres\n  user: reader\n").expect_err("no database");
    assert!(matches!(error, LoadError::Invalid(_)));
}

#[test]
fn a_sqlite_entry_needs_a_path() {
    let error = parse("- name: s\n  kind: sqlite\n").expect_err("no path");
    assert!(matches!(error, LoadError::Invalid(_)));
}

#[test]
fn two_entries_with_one_name_are_refused() {
    let text =
        "- name: a\n  kind: sqlite\n  path: /x.db\n- name: a\n  kind: sqlite\n  path: /y.db\n";
    assert!(matches!(parse(text), Err(LoadError::Invalid(_))));
}

#[test]
fn password_env_must_name_a_variable() {
    let text =
        "- name: pg\n  kind: postgres\n  database: a\n  user: b\n  password_env: not a name\n";
    assert!(matches!(parse(text), Err(LoadError::Invalid(_))));
}

#[test]
fn a_file_on_disk_is_read_and_checked_like_its_text() {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let file = dir.path().join("databases.yaml");
    std::fs::write(&file, VALID).expect("write the file");
    let specs = load(&file).expect("a valid file");
    assert_eq!(specs, parse(VALID).expect("valid entries"));
}

#[test]
fn a_file_may_be_as_big_as_the_limit_and_no_bigger() {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let file = dir.path().join("databases.yaml");
    let entry = "- name: a\n  kind: sqlite\n  path: /x.db\n";
    let padding = usize::try_from(MAX_BYTES).expect("a size") - entry.len() - 1;
    let at_the_limit = format!("{entry}#{}\n", "x".repeat(padding - 1));
    assert_eq!(u64::try_from(at_the_limit.len()), Ok(MAX_BYTES));
    std::fs::write(&file, &at_the_limit).expect("write the file");
    assert_eq!(load(&file).expect("at the limit").len(), 1);

    std::fs::write(&file, format!("{at_the_limit}#\n")).expect("write the file");
    assert_eq!(load(&file), Err(LoadError::Unreadable));
}

#[test]
fn text_that_is_not_utf8_is_unreadable() {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let file = dir.path().join("databases.yaml");
    std::fs::write(&file, [0xff, 0xfe, 0xfd]).expect("write the file");
    assert_eq!(load(&file), Err(LoadError::Unreadable));
}

#[test]
fn a_path_the_system_refuses_to_open_is_unreadable_not_missing() {
    assert_eq!(
        load(Path::new("databases\0.yaml")),
        Err(LoadError::Unreadable)
    );
}

#[test]
fn every_refusal_names_its_rule() {
    let cases = [
        (LoadError::Missing, "no databases.yaml"),
        (LoadError::Unreadable, "databases.yaml cannot be read"),
        (
            LoadError::Malformed { line: Some(3) },
            "databases.yaml line 3 is not a valid connection (a password goes in password_env, never in this file)",
        ),
        (
            LoadError::Malformed { line: None },
            "databases.yaml is not a list of connections",
        ),
        (
            LoadError::Invalid("two connections share a name"),
            "databases.yaml: two connections share a name",
        ),
    ];
    for (error, text) in cases {
        assert_eq!(error.to_string(), text);
    }
}

#[test]
fn a_file_that_is_not_a_list_is_refused() {
    let error = parse("name: pg\nkind: sqlite\n").expect_err("a mapping is not a list");
    assert!(matches!(error, LoadError::Malformed { .. }));
}

#[test]
fn a_connection_needs_a_name() {
    let error = parse("- name: \"  \"\n  kind: sqlite\n  path: /x.db\n").expect_err("no name");
    assert_eq!(error, LoadError::Invalid("a connection needs a name"));
}
