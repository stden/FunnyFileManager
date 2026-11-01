//! Purpose: Parse an entire input batch before any part of it can affect a session.
use super::Mouse;
use crate::{action::KeyChord, automation::options::invalid};
use crossterm::event::{Event, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use std::io;

pub(in crate::automation) fn events(
    keys: Vec<String>,
    text: Option<String>,
    mouse: Option<Mouse>,
) -> io::Result<Vec<Event>> {
    if keys.len() > 512 {
        return Err(invalid("too many keys in an input batch"));
    }
    let mut events: Vec<_> = keys
        .into_iter()
        .map(|key| {
            key.parse::<KeyChord>()
                .map(|chord| Event::Key(chord.to_event()))
                .map_err(|_error| invalid("invalid automation key"))
        })
        .collect::<io::Result<_>>()?;
    if let Some(text) = text {
        events.push(Event::Paste(text));
    }
    events.extend(
        mouse
            .map(|mouse| mouse_event(&mouse))
            .transpose()?
            .map(Event::Mouse),
    );
    Ok(events)
}

fn mouse_event(mouse: &Mouse) -> io::Result<MouseEvent> {
    Ok(MouseEvent {
        kind: mouse_kind(&mouse.kind, button(mouse.button.as_deref())?)?,
        column: mouse.column,
        row: mouse.row,
        modifiers: modifiers(&mouse.modifiers)?,
    })
}

fn button(button: Option<&str>) -> io::Result<MouseButton> {
    match button.unwrap_or("left") {
        "left" => Ok(MouseButton::Left),
        "right" => Ok(MouseButton::Right),
        "middle" => Ok(MouseButton::Middle),
        _ => Err(invalid("invalid mouse button")),
    }
}

fn mouse_kind(kind: &str, button: MouseButton) -> io::Result<MouseEventKind> {
    Ok(match kind {
        "down" => MouseEventKind::Down(button),
        "up" => MouseEventKind::Up(button),
        "drag" => MouseEventKind::Drag(button),
        "move" | "moved" => MouseEventKind::Moved,
        "scroll_up" => MouseEventKind::ScrollUp,
        "scroll_down" => MouseEventKind::ScrollDown,
        "scroll_left" => MouseEventKind::ScrollLeft,
        "scroll_right" => MouseEventKind::ScrollRight,
        _ => return Err(invalid("invalid mouse event")),
    })
}

fn modifiers(modifiers: &[String]) -> io::Result<KeyModifiers> {
    modifiers
        .iter()
        .try_fold(KeyModifiers::empty(), |flags, modifier| {
            let flag = match modifier.as_str() {
                "ctrl" | "control" => KeyModifiers::CONTROL,
                "alt" => KeyModifiers::ALT,
                "shift" => KeyModifiers::SHIFT,
                _ => return Err(invalid("invalid mouse modifier")),
            };
            Ok(flags | flag)
        })
}
