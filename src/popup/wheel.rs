//! Purpose: The mouse wheel over a screen: that screen's own scroll command where it scrolls, and left to the dialog protocol elsewhere.

use crossterm::event::KeyModifiers;

use crate::models::{App, Overlay};

/// The wheel over the viewer, the editor's text or the task list's table is
/// that screen's own scroll command. `false` over anything else — a dialog,
/// a menu, a list, the editor's and the task list's own dialogs — where it
/// is the Up or Down key of the dialog protocol, for the caller to press.
pub fn wheel(app: &mut App, wheel: crate::mouse::Wheel, modifiers: KeyModifiers) -> bool {
    match app.overlay {
        Overlay::Viewer(_) => super::viewer::wheel(app, wheel),
        Overlay::Editor(_) => super::editor::wheel(app, wheel, modifiers),
        Overlay::Tasks(_) => super::tasks::wheel(app, wheel),
        _ => false,
    }
}
