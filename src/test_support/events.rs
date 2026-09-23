//! Shared deterministic input and rendered-screen helpers for crate unit tests.

use super::WritableTestBackend;
#[cfg(feature = "test-helpers")]
use crate::models::Panel;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Terminal;

pub fn key_event(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}

pub fn key_event_with_mods(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

pub fn buffer_text(terminal: &Terminal<WritableTestBackend>) -> String {
    buffer_string(terminal.backend().buffer())
}

pub fn buffer_string(buf: &ratatui::buffer::Buffer) -> String {
    let mut text = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            text.push_str(buf[(x, y)].symbol());
        }
        text.push('\n');
    }
    text
}

#[cfg(feature = "test-helpers")]
pub fn select_entry(panel: &mut Panel, name: &str) {
    if let Some(index) = panel.entries.iter().position(|entry| entry.name == name) {
        panel.select(Some(index));
    }
}
