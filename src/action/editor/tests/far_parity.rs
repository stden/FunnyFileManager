//! Purpose: the editor keys that Far Manager's editor has and FFM binds to the
//! same caret moves or clipboard actions. The Far keys FFM has no command for
//! are in `far_gaps`.

use crossterm::event::KeyCode;

use super::{command, moves, ALT, CTRL, SHIFT};
use crate::action::editor::{EditorCommand, EditorNavigation};
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

/// FarEng.hlf.m4, "Block operations" (line 3198): Shift+Del and Ctrl+X cut
/// the selected block to the clipboard.
#[test]
fn ctrl_x_and_shift_del_cut_the_selection() {
    assert_eq!(command(KeyCode::Char('x'), CTRL), EditorCommand::Cut);
    assert_eq!(command(KeyCode::Delete, SHIFT), EditorCommand::Cut);
    assert_eq!(
        command(KeyCode::Delete, CTRL.union(SHIFT)),
        EditorCommand::Ignore
    );
    assert_eq!(
        command(KeyCode::Delete, ALT.union(SHIFT)),
        EditorCommand::Ignore
    );
}
