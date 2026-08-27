//! Unit tests: what the delete confirmation dialog actually prints for the
//! paths it is about to remove.
//!
//! The existing dialog tests only check that rendering does not panic; these
//! assert the cells, so a wrong count or a swallowed path name is caught.

use super::*;

use ratatui::backend::TestBackend;
use ratatui::Terminal;

use std::path::PathBuf;

use tempfile::TempDir;

const SCREEN_W: u16 = 80;
const SCREEN_H: u16 = 40;

/// Delete-confirmation rectangle — the same `centered_rect` call the renderer
/// makes.
fn dialog_area() -> Rect {
    centered_rect(60, 40, Rect::new(0, 0, SCREEN_W, SCREEN_H))
}

/// App whose panels list empty directories, so no real file name from the
/// working directory can satisfy an assertion by accident.
fn app_on_empty_dirs() -> (App, TempDir, TempDir) {
    let left = tempfile::tempdir().unwrap();
    let right = tempfile::tempdir().unwrap();
    let mut app = App::new().unwrap();
    app.left_panel = Panel::new(left.path().to_path_buf()).unwrap();
    app.right_panel = Panel::new(right.path().to_path_buf()).unwrap();
    (app, left, right)
}

fn draw_delete_confirm(paths: Vec<PathBuf>) -> (Terminal<TestBackend>, TempDir, TempDir) {
    let (mut app, left, right) = app_on_empty_dirs();
    app.set_overlay(Overlay::DeleteConfirm(paths));
    let mut terminal = Terminal::new(TestBackend::new(SCREEN_W, SCREEN_H)).unwrap();
    terminal.draw(|f| ui(f, &mut app)).unwrap();
    (terminal, left, right)
}

/// `width` cells of row `y`, starting at column `x`.
fn cells(terminal: &Terminal<TestBackend>, y: u16, x: u16, width: u16) -> String {
    let buf = terminal.backend().buffer();
    (x..x + width).map(|col| buf[(col, y)].symbol()).collect()
}

#[test]
fn test_delete_confirm_counts_the_items_and_lists_their_names() {
    let (terminal, _left, _right) = draw_delete_confirm(vec![
        PathBuf::from("/tmp/alpha.txt"),
        PathBuf::from("/tmp/beta.txt"),
        PathBuf::from("/tmp/gamma.txt"),
    ]);
    let dialog = dialog_area();

    assert_eq!(
        cells(&terminal, dialog.y + 1, dialog.x + 1, 18),
        "Delete 3 item(s)? ",
        "header row"
    );
    // Header, blank spacer, then one row per file — names only, not full paths.
    for (offset, name) in ["  alpha.txt", "  beta.txt", "  gamma.txt"]
        .into_iter()
        .enumerate()
    {
        let row = dialog.y + 3 + u16::try_from(offset).unwrap();
        assert_eq!(
            cells(
                &terminal,
                row,
                dialog.x + 1,
                u16::try_from(name.len()).unwrap()
            ),
            name,
            "preview row {offset}"
        );
    }
}

/// A path whose last component is not a file name (a trailing `..`, a bare
/// root) has no `file_name()`; the dialog must still say what it would delete.
#[test]
fn test_delete_confirm_falls_back_to_the_full_path_without_a_file_name() {
    let (terminal, _left, _right) = draw_delete_confirm(vec![PathBuf::from("/tmp/project/..")]);
    let dialog = dialog_area();

    assert_eq!(
        cells(&terminal, dialog.y + 3, dialog.x + 1, 18),
        "  /tmp/project/.. ",
        "preview row must show the whole path"
    );
}

#[test]
fn test_delete_confirm_summarizes_the_files_past_the_first_five() {
    let paths: Vec<PathBuf> = (0..8)
        .map(|i| PathBuf::from(format!("/tmp/file{i}.txt")))
        .collect();
    let (terminal, _left, _right) = draw_delete_confirm(paths);
    let dialog = dialog_area();

    // Header, spacer, five preview rows, then the summary.
    assert_eq!(
        cells(&terminal, dialog.y + 8, dialog.x + 1, 18),
        "  ... and 3 more  ",
        "summary row"
    );
}
