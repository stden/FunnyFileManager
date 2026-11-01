//! Purpose: The hang-up flag before any signal. Raising SIGHUP here would set
//! it for every test in this process, and installing the handlers would let
//! the runner outlive a SIGTERM; the pty session tests send the real signals
//! to the real binary instead.

use super::hung_up;

#[test]
fn nothing_has_ended_the_session_before_a_signal_arrives() {
    assert!(!hung_up());
}
