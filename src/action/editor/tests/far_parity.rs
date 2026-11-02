//! Purpose: the editor keys that Far Manager's editor has and FFM binds to the
//! same caret moves. The Far keys FFM has no command for are in `far_gaps`.

use crossterm::event::KeyCode;

use super::{command, moves, CTRL};
use crate::action::editor::EditorNavigation;
use crate::models::SelectionMode;

/// FarEng.hlf.m4, "Cursor movement": Ctrl+PgUp and Ctrl+PgDn go to the start
/// and the end of the file, as Ctrl+Home and Ctrl+End do.
#[test]
fn ctrl_page_keys_go_to_the_ends_of_the_file() {
    assert_eq!(
        command(KeyCode::PageUp, CTRL),
        moves(EditorNavigation::DocumentStart, SelectionMode::Move)
    );
    assert_eq!(
        command(KeyCode::PageDown, CTRL),
        moves(EditorNavigation::DocumentEnd, SelectionMode::Move)
    );
}
