//! Purpose: Pin where a password comes from and what an error may say about
//! it: the value of the named variable, the name alone when it is missing,
//! and no secret in text a driver returns.

use super::{redact, resolve};
use crate::models::database::PostgresEndpoint;
use crate::ops::database::DbError;

fn endpoint(password_env: Option<&str>) -> PostgresEndpoint {
    PostgresEndpoint {
        host: "localhost".to_owned(),
        port: 5432,
        database: "app".to_owned(),
        user: "reader".to_owned(),
        password_env: password_env.map(str::to_owned),
    }
}

#[test]
fn an_entry_without_a_variable_has_no_password() {
    assert_eq!(
        resolve(&endpoint(None), |_| Some("unused".to_owned())),
        Ok(None)
    );
}

#[test]
fn the_named_variable_supplies_the_password() {
    let lookup = |name: &str| (name == "PG_PASS").then(|| "s3cret".to_owned());
    assert_eq!(
        resolve(&endpoint(Some("PG_PASS")), lookup),
        Ok(Some("s3cret".to_owned()))
    );
}

#[test]
fn a_missing_variable_is_reported_by_name_only() {
    let error = resolve(&endpoint(Some("PG_PASS")), |_| None).expect_err("a missing variable");
    assert_eq!(error, DbError::MissingPassword("PG_PASS".to_owned()));
    assert!(error.to_string().contains("PG_PASS"));
}

#[test]
fn redaction_takes_the_secret_out_of_driver_text() {
    let text = redact("password authentication failed for s3cret", Some("s3cret"));
    assert!(!text.contains("s3cret"));
}

#[test]
fn redaction_without_a_secret_leaves_the_text_alone() {
    assert_eq!(redact("timeout", None), "timeout");
    assert_eq!(redact("timeout", Some("")), "timeout");
}
