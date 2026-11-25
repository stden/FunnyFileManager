//! Purpose: Unit tests for `retain_first_error` and `finish_cleanup`.

use std::io;

use super::{finish_cleanup, retain_first_error};

#[test]
fn retain_first_error_keeps_the_earliest_failure_across_multiple_steps() {
    let mut first_error = None;
    assert!(retain_first_error(&mut first_error, Ok(())));
    assert!(first_error.is_none());

    assert!(!retain_first_error(
        &mut first_error,
        Err(io::Error::other("first"))
    ));
    assert!(!retain_first_error(
        &mut first_error,
        Err(io::Error::other("second"))
    ));

    let retained = finish_cleanup(first_error).unwrap_err();
    assert_eq!(retained.to_string(), "first");
    assert!(finish_cleanup(None).is_ok());
}
