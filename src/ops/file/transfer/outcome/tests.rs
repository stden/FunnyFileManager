//! Purpose: `BatchOutcome` keeps the first refusal and records subsequent errors.

use std::{io, path::Path};

use super::BatchOutcome;

#[test]
fn recording_a_second_error_preserves_the_first_denial() {
    let mut outcome = BatchOutcome::default();
    let first = Path::new("first.txt");
    let second = Path::new("second.txt");

    outcome.record(
        first,
        Err(io::Error::new(io::ErrorKind::PermissionDenied, "denied")),
    );
    let initial_denial = outcome.denial.clone();
    outcome.record(second, Err(io::Error::other("later failure")));

    assert!(outcome.stopped());
    assert_eq!(outcome.errors.len(), 2);
    assert_eq!(outcome.denial, initial_denial);
}
