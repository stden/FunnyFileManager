//! Unit tests: the Space analysis keys. The cursor moves among the rows, Enter
//! selects the entry in the active panel and closes, Esc and F10 close, and any
//! other key is consumed rather than reaching the panels behind the dialog.

use std::fs;

use crossterm::event::KeyCode;
use tempfile::TempDir;

use crate::models::space_analysis::SpaceAnalysisDialog;
use crate::models::{App, Overlay, Panel};
use crate::popup::process_popup_input;
use crate::popup::tests::key;

fn opened() -> (TempDir, App) {
    let dir = TempDir::new().expect("temp dir");
    for (name, bytes) in [("big.bin", 300), ("mid.bin", 200), ("small.bin", 100)] {
        fs::write(dir.path().join(name), vec![b'x'; bytes]).expect("test setup");
    }
    let mut app = App::with_panels(
        Panel::new(dir.path().to_path_buf()).expect("panel"),
        Panel::new(dir.path().to_path_buf()).expect("panel"),
    );
    crate::ops::space_analysis::open(&mut app).expect("the dialog opens");
    (dir, app)
}

fn dialog(app: &App) -> &SpaceAnalysisDialog {
    match &app.overlay {
        Overlay::SpaceAnalysis(dialog) => dialog,
        _ => panic!("the dialog is not open"),
    }
}

fn press(app: &mut App, code: KeyCode) {
    assert!(process_popup_input(app, key(code)), "{code:?} is consumed");
}

#[test]
fn the_arrows_and_home_and_end_move_the_cursor_among_the_rows() {
    let (_dir, mut app) = opened();
    assert_eq!(dialog(&app).selected_name(), Some("big.bin"));
    press(&mut app, KeyCode::Down);
    assert_eq!(dialog(&app).selected_name(), Some("mid.bin"));
    press(&mut app, KeyCode::End);
    assert_eq!(dialog(&app).selected_name(), Some("small.bin"));
    press(&mut app, KeyCode::Down);
    assert_eq!(
        dialog(&app).selected_name(),
        Some("small.bin"),
        "stops at the end"
    );
    press(&mut app, KeyCode::Up);
    press(&mut app, KeyCode::Home);
    assert_eq!(dialog(&app).selected_name(), Some("big.bin"));
}

#[test]
fn paging_moves_by_a_page_and_stops_at_the_ends() {
    let (_dir, mut app) = opened();
    press(&mut app, KeyCode::PageDown);
    assert_eq!(dialog(&app).cursor, 2, "three rows, so the last one");
    press(&mut app, KeyCode::PageUp);
    assert_eq!(dialog(&app).cursor, 0);
}

#[test]
fn enter_selects_the_entry_in_the_active_panel_and_closes() {
    let (_dir, mut app) = opened();
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    assert!(!app.overlay.is_active(), "Enter closes the dialog");
    let panel = app.get_active_panel_ref();
    let selected = panel.selected().and_then(|index| panel.entries.get(index));
    assert_eq!(selected.map(|entry| entry.name.as_str()), Some("mid.bin"));
}

/// An empty folder has no row to select, but Enter still closes the dialog.
#[test]
fn enter_on_an_empty_folder_only_closes_the_dialog() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = App::with_panels(
        Panel::new(dir.path().to_path_buf()).expect("panel"),
        Panel::new(dir.path().to_path_buf()).expect("panel"),
    );
    crate::ops::space_analysis::open(&mut app).expect("the dialog opens");
    press(&mut app, KeyCode::Enter);
    assert!(!app.overlay.is_active());
}

#[test]
fn esc_and_f10_close_and_other_keys_are_consumed() {
    let (_dir, mut app) = opened();
    press(&mut app, KeyCode::Char('x'));
    assert!(
        matches!(app.overlay, Overlay::SpaceAnalysis(_)),
        "other keys stay"
    );
    press(&mut app, KeyCode::Esc);
    assert!(!app.overlay.is_active());

    let (_dir, mut app) = opened();
    press(&mut app, KeyCode::F(10));
    assert!(!app.overlay.is_active(), "F10 closes too");
}
