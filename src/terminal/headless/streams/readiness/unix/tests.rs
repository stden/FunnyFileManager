//! Purpose: Unit tests for `interpret_poll` signal-interruption and error branches.

use super::interpret_poll;
use mio::Events;
use std::io;

#[test]
fn interpret_poll_treats_interrupted_as_idle_and_propagates_other_errors() {
    let events = Events::with_capacity(1);
    assert!(!interpret_poll(Err(io::ErrorKind::Interrupted.into()), &events).expect("interrupted"));
    assert!(interpret_poll(Err(io::Error::other("poll failed")), &events).is_err());
    assert!(!interpret_poll(Ok(()), &events).expect("empty events"));
}
