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

/// The row of `screen` that shows `caption`: from the caption to the right
/// edge of its frame, with the blanks and the border (`║` or `│`) dropped.
///
/// # Panics
///
/// When no row shows `caption`.
pub fn framed_row(screen: &str, caption: &str) -> String {
    let rest = screen
        .lines()
        .find_map(|line| line.split_once(caption).map(|(_, rest)| rest))
        .unwrap_or_else(|| panic!("no row shows {caption:?}\n{screen}"));
    let inside = rest.trim_end().trim_end_matches(['║', '│']).trim_end();
    format!("{caption}{inside}")
}

/// `name` as the last component of a path drawn on this platform.
pub fn last_component(name: &str) -> String {
    format!("{}{name}", std::path::MAIN_SEPARATOR)
}

#[cfg(feature = "test-helpers")]
pub fn select_entry(panel: &mut Panel, name: &str) {
    if let Some(index) = panel.entries.iter().position(|entry| entry.name == name) {
        panel.select(Some(index));
    }
}
