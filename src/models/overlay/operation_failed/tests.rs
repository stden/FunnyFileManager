//! Purpose: A stopped operation records why it stopped, and offers one retry.

use std::path::PathBuf;

use super::{Denial, ElevatedRetry, OperationFailed};
use crate::progress::Operation;
use crate::status::FailedEntry;

fn stopped(denial: Option<Denial>, retry: bool) -> OperationFailed {
    let failure = OperationFailed::new(
        Operation::Delete,
        FailedEntry::new("/srv/x/a.txt", &"Permission denied"),
        1,
        2,
    )
    .because(denial);
    if retry {
        failure.offering(Some(ElevatedRetry {
            operation: Operation::Delete,
            sources: vec![PathBuf::from("/srv/x/a.txt")],
            base: PathBuf::from("/srv/x"),
            selected: 1,
            settings: None,
        }))
    } else {
        failure.offering(None)
    }
}

#[test]
fn because_records_the_denial_and_a_retry_can_be_taken_once() {
    let mut failure = stopped(Some(Denial::NeedsAdministrator), true);
    assert_eq!(failure.denial(), Some(&Denial::NeedsAdministrator));
    assert!(failure.retry_focused());
    failure.toggle_focus();
    assert!(!failure.retry_focused());
    failure.toggle_focus();
    assert!(failure.take_retry().is_some());
    assert!(failure.take_retry().is_none());
    assert!(!failure.retry_focused());
}

#[test]
fn toggle_focus_does_nothing_without_a_retry() {
    let mut failure = stopped(None, false);
    assert!(!failure.retry_focused());
    failure.toggle_focus();
    assert!(!failure.retry_focused());
    assert!(failure.take_retry().is_none());
}
