//! Purpose: Keys for a panel that has become a quick view (`Ctrl+Q`).
//!
//! The dispatcher asks here first while the active panel is a preview;
//! `true` means the key was the preview's. Only the keys of the viewer's
//! scroll commands are, as the keymap's `viewer` bindings give them: Far lets
//! "many of the internal viewer commands" work on the previewed file, and
//! these are the ones a panel-sized window needs. Everything else falls
//! through, so `Tab` still leaves, `Ctrl+Q` still turns the preview off and
//! `F10` still quits.

use crossterm::event::KeyEvent;

use super::viewer::scroll_body;
use crate::action::viewer::{resolve, ViewerCommand, ViewerScroll};
use crate::models::{App, PanelKind, QuickView};

/// Whether the quick-view panel consumed `key`.
pub fn process(app: &mut App, key: KeyEvent) -> bool {
    match resolve(&app.keymap.viewer, key) {
        Some(ViewerCommand::Scroll(motion)) => scroll(app, motion),
        // The viewer's closing keys are the panel's here: by default Enter
        // opens, F3 views, F10 asks to quit.
        Some(ViewerCommand::Close) | None => false,
    }
}

/// Scroll the active quick view; `false` when the active panel is not one.
pub fn scroll(app: &mut App, motion: ViewerScroll) -> bool {
    let page = QuickView::page_for(app.viewport);
    let PanelKind::QuickView(view) = &mut app.get_active_panel().kind else {
        return false;
    };
    scroll_body(&mut **view, motion, page);
    true
}

#[cfg(test)]
mod tests;
