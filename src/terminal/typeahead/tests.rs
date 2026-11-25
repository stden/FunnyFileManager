//! Purpose: The type-ahead drain notices a cancel, and stops at the first
//! failed read or poll without reading further.

use super::*;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

#[test]
fn a_drain_notices_cancel_and_stops_when_reading_fails() {
    let mut events = [
        Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
    ]
    .into_iter();
    let cancelled = discard_from(
        || Ok(true),
        || {
            events
                .next()
                .ok_or_else(|| std::io::Error::other("terminal closed"))
        },
    );
    assert!(cancelled);
    assert!(events.next().is_none());
}

#[test]
fn failed_polling_does_not_attempt_a_read() {
    assert!(!discard_from(
        || Err(std::io::Error::other("terminal unavailable")),
        || panic!("a failed poll must not read"),
    ));
}
