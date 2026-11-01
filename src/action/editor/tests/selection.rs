//! Contract tests for the caret's selection mode.

use crossterm::event::KeyModifiers;

use crate::action::editor::selection_mode;
use crate::models::SelectionMode;

/// Shift alone turns a caret move into a selection; no other modifier does.
#[test]
fn selection_mode_extends_only_with_shift() {
    assert_eq!(selection_mode(KeyModifiers::SHIFT), SelectionMode::Extend);
    assert_eq!(selection_mode(KeyModifiers::NONE), SelectionMode::Move);
    assert_eq!(selection_mode(KeyModifiers::CONTROL), SelectionMode::Move);
    assert_eq!(
        selection_mode(KeyModifiers::CONTROL | KeyModifiers::SHIFT),
        SelectionMode::Extend
    );
}
