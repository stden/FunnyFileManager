//! Purpose: The hang-up flag before any signal. Raising SIGHUP here would set
//! it for every test in this process, and installing the handlers would let
//! the runner outlive a SIGTERM; the pty session tests send the real signals
//! to the real binary instead.

use super::{clone_outcome, hung_up};
use std::io;

#[test]
fn nothing_has_ended_the_session_before_a_signal_arrives() {
    assert!(!hung_up());
}

#[test]
fn clone_outcome_preserves_success_and_maps_cached_errors() {
    assert!(clone_outcome(&Ok(())).is_ok());
    let err = clone_outcome(&Err(io::Error::other("sigaction denied"))).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::Other);
    assert!(err.to_string().contains("sigaction denied"));
}
