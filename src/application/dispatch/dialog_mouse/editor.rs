//! Purpose: Handle mouse clicks on the F4 editor body — move the cursor to
//! the clicked line and column.

use std::io::Write;

use ratatui::backend::Backend;
use ratatui::Terminal;

use crate::models::{App, Overlay};

/// Rows the editor spends on chrome (the top bar).
const EDITOR_BAR_ROWS: usize = 1;

/// Handle a left-button press anywhere in the F4 editor's text area.
///
/// Returns `true` when the overlay is an editor and the click was consumed
/// (including a click on the bar row, which is noted but does not move the
/// cursor).  Returns `false` when the overlay is not an editor, or when a
/// modal subdialog (search, exit confirm, …) is up and its buttons should
/// handle the click instead.
pub(super) fn click_editor_body<B: Backend + Write>(
    terminal: &Terminal<B>,
    app: &mut App,
    column: u16,
    row: u16,
) -> bool {
    // Only handle clicks when the editor is open.
    if !matches!(app.overlay, Overlay::Editor(_)) {
        return false;
    }
    // Let dialog buttons handle clicks while a modal subdialog is up.
    if app.get_editor().is_some_and(|e| e.modal().is_some()) {
        return false;
    }
    let size = terminal.size().unwrap_or_default();
    let height = usize::from(size.height);
    let width = usize::from(size.width);
    // A degenerate terminal or a click on the bar row: consume but do not move.
    if height <= EDITOR_BAR_ROWS || usize::from(row) < EDITOR_BAR_ROWS {
        return true;
    }
    if let Some(editor) = app.get_editor_mut() {
        let (scroll_y, scroll_x) = editor.viewport(height, width);
        let target_line = usize::from(row) - EDITOR_BAR_ROWS + scroll_y;
        let target_cell = usize::from(column) + scroll_x;
        editor.jump_to_screen_position(target_line, target_cell);
    }
    true
}
