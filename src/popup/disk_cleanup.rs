//! Purpose: Key handling for the Disk Cleanup overlay (`:cleanup`).
//!
//! Space / Enter toggle checkboxes; Enter on a button activates it;
//! arrows and Tab navigate; Esc / F10 cancel.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::models::App;
use crate::ops::disk_cleanup as op;

pub(super) fn process(app: &mut App, code: KeyCode, _modifiers: KeyModifiers) -> bool {
    let Some(field) = op::current_field(app) else {
        return false;
    };

    match code {
        KeyCode::Esc | KeyCode::F(10) => op::cancel(app),

        KeyCode::Enter => match field {
            crate::models::CleanupField::Run => op::run(app),
            crate::models::CleanupField::Cancel => op::cancel(app),
            crate::models::CleanupField::Item(_) => op::toggle_and_advance(app),
        },

        KeyCode::Char(' ') => op::space_checked(app),

        KeyCode::Down | KeyCode::Tab => op::move_focus(app, true),
        KeyCode::Up | KeyCode::BackTab => op::move_focus(app, false),

        _ => {}
    }
    true
}
