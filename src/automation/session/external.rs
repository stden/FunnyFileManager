//! Purpose: Forward input to a private external terminal instead of queuing it behind a running program.
use crate::terminal::headless::{ExternalState, HeadlessHost};
use crossterm::event::{Event, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use serde_json::{json, Value};
use std::io;

pub(super) fn snapshot(host: &HeadlessHost) -> Value {
    let captured = host.snapshot();
    let size = host.size();
    let (state, code) = match captured.state {
        ExternalState::Idle => ("idle", None),
        ExternalState::Running => ("running", None),
        ExternalState::Exited { code } => ("exited", Some(code)),
        ExternalState::Cancelled => ("cancelled", None),
        ExternalState::Failed => ("failed", None),
    };
    json!({"state": state, "code": code, "output": captured.output, "dropped_bytes": captured.dropped_bytes,
        "width": size.width, "height": size.height})
}

pub(super) fn input(host: &HeadlessHost, events: Vec<Event>) -> io::Result<()> {
    for event in events {
        match event {
            Event::Key(key) => {
                host.key(key)?;
            }
            Event::Paste(text) => {
                host.input(&text)?;
            }
            Event::Mouse(mouse) => {
                host.input(&mouse_sequence(mouse))?;
            }
            Event::FocusGained | Event::FocusLost | Event::Resize(_, _) => {}
        }
    }
    Ok(())
}

fn mouse_sequence(mouse: MouseEvent) -> String {
    let (button, end) = match mouse.kind {
        MouseEventKind::Down(button) => (button_code(button), 'M'),
        MouseEventKind::Up(button) => (button_code(button), 'm'),
        MouseEventKind::Drag(button) => (button_code(button) + 32, 'M'),
        MouseEventKind::Moved => (35, 'M'),
        MouseEventKind::ScrollUp => (64, 'M'),
        MouseEventKind::ScrollDown => (65, 'M'),
        MouseEventKind::ScrollLeft => (66, 'M'),
        MouseEventKind::ScrollRight => (67, 'M'),
    };
    let modifiers = [
        (KeyModifiers::SHIFT, 4),
        (KeyModifiers::ALT, 8),
        (KeyModifiers::CONTROL, 16),
    ]
    .iter()
    .filter(|(flag, _)| mouse.modifiers.contains(*flag))
    .map(|(_, value)| value)
    .sum::<u16>();
    format!(
        "\u{1b}[<{};{};{}{end}",
        button + modifiers,
        u32::from(mouse.column) + 1,
        u32::from(mouse.row) + 1
    )
}

const fn button_code(button: MouseButton) -> u16 {
    match button {
        MouseButton::Left => 0,
        MouseButton::Middle => 1,
        MouseButton::Right => 2,
    }
}

#[cfg(test)]
mod tests;
