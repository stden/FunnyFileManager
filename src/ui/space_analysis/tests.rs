//! Unit tests: the Space analysis dialog draws its title, its totals, one row per
//! entry with the verdict and its reason, the cursor on the first row, and the
//! keys it answers to.

use std::fs;

use ratatui::Terminal;
use tempfile::TempDir;

use crate::models::{App, Panel};
use crate::test_support::{buffer_text, WritableTestBackend};
use crate::ui::render_overlays;

fn opened(dir: &TempDir) -> App {
    fs::create_dir_all(dir.path().join("__pycache__")).expect("test setup");
    fs::write(dir.path().join("__pycache__/mod.pyc"), vec![b'x'; 2048]).expect("test setup");
    fs::write(dir.path().join("notes.txt"), b"hello").expect("test setup");
    let mut app = App::with_panels(
        Panel::new(dir.path().to_path_buf()).expect("panel"),
        Panel::new(dir.path().to_path_buf()).expect("panel"),
    );
    crate::ops::space_analysis::open(&mut app).expect("the dialog opens");
    app
}

#[test]
fn the_dialog_draws_its_totals_rows_verdicts_and_keys() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = opened(&dir);
    let mut terminal = Terminal::new(WritableTestBackend::new(110, 40)).expect("terminal");
    terminal
        .draw(|f| render_overlays(f, &mut app))
        .expect("draw");
    let screen = buffer_text(&terminal);

    assert!(screen.contains("Space analysis"), "{screen}");
    assert!(screen.contains("Reclaimable safely"), "{screen}");
    assert!(screen.contains("SAFE"), "{screen}");
    assert!(screen.contains("REVIEW"), "{screen}");
    assert!(screen.contains("not under git"), "{screen}");
    assert!(screen.contains("Enter: select in the panel"), "{screen}");
    assert!(screen.contains("Esc: close"), "{screen}");
}

#[test]
fn the_cursor_marks_the_largest_row() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = opened(&dir);
    let mut terminal = Terminal::new(WritableTestBackend::new(110, 40)).expect("terminal");
    terminal
        .draw(|f| render_overlays(f, &mut app))
        .expect("draw");
    let screen = buffer_text(&terminal);
    let first_row = screen
        .lines()
        .find(|line| line.contains("__pycache__"))
        .unwrap_or_else(|| panic!("no row for the cache: {screen}"));
    assert!(first_row.contains('>'), "{first_row:?}");
}
