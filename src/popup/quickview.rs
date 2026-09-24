//! Purpose: Keys for a panel that has become a quick view (`Ctrl+Q`).
//!
//! The dispatcher asks here first while the active panel is a preview;
//! `true` means the key was the preview's. Only the viewer's scrolling keys
//! are: Far lets "many of the internal viewer commands" work on the previewed
//! file, and these are the ones a panel-sized window needs. Everything else
//! falls through, so `Tab` still leaves, `Ctrl+Q` still turns the preview off
//! and `F10` still quits.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::models::{App, PanelKind, QuickView};
use crate::scroll::WIDE_STEP;

/// Whether the quick-view panel consumed `key`.
pub fn process(app: &mut App, key: KeyEvent) -> bool {
    let page = QuickView::page_for(app.viewport);
    let PanelKind::QuickView(view) = &mut app.get_active_panel().kind else {
        return false;
    };
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    match key.code {
        KeyCode::Down => view.scroll_down(1, page),
        KeyCode::Up => view.scroll_up(1),
        KeyCode::PageDown => view.scroll_down(page, page),
        KeyCode::PageUp => view.scroll_up(page),
        KeyCode::Home => view.scroll_to_top(),
        KeyCode::End => view.scroll_to_bottom(page),
        KeyCode::Left if ctrl => view.scroll_left(WIDE_STEP),
        KeyCode::Right if ctrl => view.scroll_right(WIDE_STEP),
        KeyCode::Left => view.scroll_left(1),
        KeyCode::Right => view.scroll_right(1),
        _ => return false,
    }
    true
}

#[cfg(test)]
mod tests;
