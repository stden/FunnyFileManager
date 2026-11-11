//! Purpose: Exercises end-to-end event-loop behavior with an injected terminal backend.
//!

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, Terminal};
use std::{io, sync::mpsc, time::Duration};

#[path = "integration_test/entry_points.rs"]
mod entry_points;
#[path = "integration_test/keys.rs"]
mod keys;
#[path = "integration_test/workflows.rs"]
mod workflows;

// Test helper to simulate key events
const fn simulate_key_event(key: KeyCode, modifiers: KeyModifiers) -> Event {
    Event::Key(KeyEvent::new(key, modifiers))
}
