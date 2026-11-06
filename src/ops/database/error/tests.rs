//! Purpose: Pin the text of a database error: the driver's own words, or the name
//! of the variable a missing password should have been in, never a password.

use super::DbError;

#[test]
fn a_driver_error_shows_the_drivers_text_unchanged() {
    assert_eq!(
        DbError::Driver("connection refused".to_owned()).to_string(),
        "connection refused"
    );
}

#[test]
fn a_missing_password_names_the_variable_that_is_unset() {
    assert_eq!(
        DbError::MissingPassword("WAREHOUSE_PASSWORD".to_owned()).to_string(),
        "environment variable WAREHOUSE_PASSWORD is not set"
    );
}
