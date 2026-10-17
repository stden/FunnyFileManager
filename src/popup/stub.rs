//! Purpose: The one behaviour every not-yet-built dialog shares — Esc and F10
//! take it down, and every other key is consumed rather than falling through
//! to the panels behind it.
//!
//! Shared so that twenty placeholder dialogs are one function to get right.
//! A feature replaces its own `process` with the real keys and stops calling
//! this; when the last one does, this file goes.

use crossterm::event::KeyCode;

use crate::models::App;

pub(super) fn close_on_escape(app: &mut App, code: KeyCode) -> bool {
    if matches!(code, KeyCode::Enter | KeyCode::Esc | KeyCode::F(10)) {
        app.close_overlay();
    }
    true
}
