//! Purpose: Keyboard routing for Jira, HTTP MCP services, and AI assistant overlays.

mod ai;
mod jira;
mod mcp_remote;
#[cfg(test)]
mod tests;

use crossterm::event::{KeyCode, KeyModifiers};

use crate::lineedit::LineEditor;
use crate::models::{App, Overlay, RowCursor, RowMotion};
use crate::popup::text_field;

/// Handle key input when a Jira, MCP services, or AI overlay is open.
pub(super) fn process(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    match &app.overlay {
        Overlay::Jira(_) => jira::process(app, code, modifiers),
        Overlay::McpServices(_) => mcp_remote::process(app, code, modifiers),
        Overlay::Ai(_) => ai::process(app, code, modifiers),
        _ => return false,
    }
    true
}

/// Map vertical list / pager navigation keys onto [`RowMotion`].
pub(super) fn motion_for(code: KeyCode) -> Option<RowMotion> {
    match code {
        KeyCode::Up => Some(RowMotion::Previous),
        KeyCode::Down => Some(RowMotion::Next),
        KeyCode::PageUp => Some(RowMotion::PageUp(12)),
        KeyCode::PageDown => Some(RowMotion::PageDown(12)),
        KeyCode::Home => Some(RowMotion::First),
        KeyCode::End => Some(RowMotion::Last),
        _ => None,
    }
}

/// Clamp and apply vertical scroll motion to a line buffer offset.
pub(super) fn scroll_lines(scroll: &mut usize, lines_len: usize, motion: RowMotion) {
    *scroll = RowCursor::at(*scroll, lines_len)
        .moved(motion, lines_len)
        .index();
}

/// Forward typing, caret motion, and clipboard chords to whichever modal line editor is active.
pub(super) fn edit_active_line(
    app: &mut App,
    code: KeyCode,
    modifiers: KeyModifiers,
    pick: fn(&mut Overlay) -> Option<&mut LineEditor>,
) {
    text_field::with_clipboard(app, |app, clipboard| {
        if let Some(line) = pick(&mut app.overlay) {
            text_field::edit(line, code, modifiers, clipboard);
        }
    });
}

/// True when neither Ctrl nor Alt modifier is held alongside a character key.
pub(super) fn plain_modifiers(modifiers: KeyModifiers) -> bool {
    !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
}
