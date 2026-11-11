//! Purpose: Pin the values the explorer carries: a name is never empty and shows
//! as it was given, and each driver names its kind.

use super::{Driver, Name, PostgresEndpoint};

#[test]
fn a_name_is_never_empty_and_shows_as_given() {
    assert!(Name::new("").is_none());
    let name = Name::new("analytics").expect("a name");
    assert_eq!(name.as_str(), "analytics");
    assert_eq!(name.to_string(), "analytics");
}

#[test]
fn each_driver_names_its_kind() {
    let sqlite = Driver::Sqlite {
        path: "/data/x.db".into(),
    };
    let postgres = Driver::Postgres(PostgresEndpoint {
        host: "localhost".to_owned(),
        port: 5432,
        database: "analytics".to_owned(),
        user: "reader".to_owned(),
        password_env: None,
    });
    assert_eq!(sqlite.kind_label(), "sqlite");
    assert_eq!(postgres.kind_label(), "postgres");
}
