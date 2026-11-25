//! Purpose: Private PTY sessions never use the protocol or the host keyboard.

use super::*;
use std::io;

#[test]
fn scoped_hosts_restore_and_cancellation_never_polls_a_terminal() {
    let outer = HeadlessHost::new();
    let inner = HeadlessHost::new();
    let _scope = outer.enter();
    assert_eq!(cancelled(), Some(false));
    {
        let _nested = inner.enter();
        inner.cancel();
        assert!(crate::terminal::cancel_requested());
        assert!(crate::terminal::discard_queued());
    }
    assert!(!crate::terminal::cancel_requested());
    outer.cancel();
    outer.reset().unwrap();
    assert!(!crate::terminal::cancel_requested());
    assert_eq!(outer.snapshot().state, ExternalState::Idle);
}

#[test]
fn inactive_input_is_rejected() {
    let host = HeadlessHost::new();
    assert_eq!(
        host.input("secret").unwrap_err().kind(),
        io::ErrorKind::NotConnected
    );
    assert_eq!(host.snapshot().output, "");
}

#[test]
fn private_input_is_bounded_and_never_enters_the_output_capture() {
    let host = HeadlessHost::new();
    let (sender, _receiver) = std::sync::mpsc::sync_channel(1);
    let generation = host.begin(sender).unwrap();
    host.input("secret").unwrap();
    assert_eq!(
        host.input("more").unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    assert_eq!(
        host.input(&"x".repeat(8193)).unwrap_err().kind(),
        io::ErrorKind::InvalidInput
    );
    assert_eq!(host.snapshot().output, "");
    host.finish(generation, ExternalState::Exited { code: 0 });
    host.reset().unwrap();
}

#[test]
fn output_is_bounded_and_old_readers_cannot_change_a_new_generation() {
    let host = HeadlessHost::new();
    let (sender, _receiver) = std::sync::mpsc::sync_channel(1);
    let generation = host.begin(sender).unwrap();
    host.append(generation, &vec![b'x'; state::OUTPUT_LIMIT + 10]);
    assert_eq!(host.snapshot().output.len(), state::OUTPUT_LIMIT);
    assert_eq!(host.snapshot().dropped_bytes, 10);
    host.finish(generation, ExternalState::Exited { code: 0 });
    host.reset().unwrap();
    host.append(generation, b"old child output");
    assert_eq!(host.snapshot().output, "");
}

mod detached;
#[cfg(unix)]
mod foreground;
#[cfg(test)]
mod process_tests;
mod resize;
