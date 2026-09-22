//! Unit tests: what the delete confirmation dialog actually prints for the
//! paths it is about to remove.
//!
//! The existing dialog tests only check that rendering does not panic; these
//! assert the cells, so a wrong count or a swallowed path name is caught.

use super::*;
use crate::models::Panel;

use ratatui::backend::TestBackend;
use ratatui::Terminal;

use std::path::PathBuf;

use tempfile::TempDir;

const SCREEN_W: u16 = 80;
const SCREEN_H: u16 = 40;

/// Far's warning ground, so a test can tell the dialog from the panels behind
/// it without knowing where it landed.
const WARNING_BG: ratatui::style::Color = ratatui::style::Color::Rgb(128, 0, 0);

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

/// The whole screen as one string: the dialog is sized to its content and
/// centred, so tests ask what reached the user rather than where it landed.
fn screen_text(terminal: &Terminal<TestBackend>) -> String {
    let buf = terminal.backend().buffer();
    (0..SCREEN_H)
        .map(|y| {
            (0..SCREEN_W)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn test_delete_confirm_names_the_single_file_it_is_about() {
    let (terminal, _left, _right) = draw_delete_confirm(vec![PathBuf::from("/tmp/fast-build.cmd")]);
    let screen = screen_text(&terminal);

    assert!(screen.contains("Delete"), "dialog title:\n{screen}");
    assert!(
        screen.contains("Do you wish to delete the file?"),
        "dialog question:\n{screen}"
    );
    // The name is what the user checks before pressing Enter; a bare count is
    // the one thing that cannot be checked.
    assert!(screen.contains("fast-build.cmd"), "file name:\n{screen}");
}

#[test]
fn test_delete_confirm_counts_the_items_and_lists_their_names() {
    let (terminal, _left, _right) = draw_delete_confirm(vec![
        PathBuf::from("/tmp/alpha.txt"),
        PathBuf::from("/tmp/beta.txt"),
        PathBuf::from("/tmp/gamma.txt"),
    ]);
    let screen = screen_text(&terminal);

    assert!(
        screen.contains("Do you wish to delete 3 items?"),
        "dialog question:\n{screen}"
    );
    for name in ["alpha.txt", "beta.txt", "gamma.txt"] {
        assert!(screen.contains(name), "{name} missing:\n{screen}");
    }
}

#[test]
fn test_delete_confirm_falls_back_to_the_full_path_without_a_file_name() {
    // `..` has no file name, so the dialog must print something rather than a
    // blank row that names nothing at all.
    let (terminal, _left, _right) = draw_delete_confirm(vec![PathBuf::from("/tmp/project/..")]);
    let screen = screen_text(&terminal);

    assert!(
        screen.contains("/tmp/project/.."),
        "whole path must be shown when there is no file name:\n{screen}"
    );
}

#[test]
fn test_delete_confirm_summarizes_the_files_past_the_first_five() {
    let paths: Vec<PathBuf> = (0..8)
        .map(|i| PathBuf::from(format!("/tmp/f{i}.txt")))
        .collect();
    let (terminal, _left, _right) = draw_delete_confirm(paths);
    let screen = screen_text(&terminal);

    assert!(screen.contains("f0.txt"), "first name:\n{screen}");
    assert!(screen.contains("f4.txt"), "fifth name:\n{screen}");
    assert!(
        !screen.contains("f5.txt"),
        "sixth name must be summarised:\n{screen}"
    );
    assert!(screen.contains("… and 3 more"), "summary row:\n{screen}");
}

/// Far marks the button Enter starts on with braces and the rest with
/// brackets, so a reflexive Enter is legible before it is pressed.
#[test]
fn test_delete_confirm_offers_a_default_delete_and_a_cancel() {
    let (terminal, _left, _right) = draw_delete_confirm(vec![PathBuf::from("/tmp/x.txt")]);
    let screen = screen_text(&terminal);

    assert!(screen.contains("{ Delete }"), "default button:\n{screen}");
    assert!(screen.contains("[ Cancel ]"), "cancel button:\n{screen}");
}

/// Every question that can lose data is red — the colour lands before the text
/// is read, which is when a reflexive Enter would otherwise already be pressed.
#[test]
fn test_delete_confirm_is_drawn_on_the_warning_ground() {
    let (terminal, _left, _right) = draw_delete_confirm(vec![PathBuf::from("/tmp/x.txt")]);
    let buf = terminal.backend().buffer();

    let red = (0..SCREEN_H)
        .flat_map(|y| (0..SCREEN_W).map(move |x| (x, y)))
        .filter(|&(x, y)| buf[(x, y)].bg == WARNING_BG)
        .count();
    assert!(red > 0, "the delete dialog must be drawn on red");
}

/// The dialog drops a shadow, which is how it reads as being *over* the panels
/// rather than cut into them.
///
/// Half-transparent: the shadowed cells keep their own characters and are
/// darkened in place. Blanking them would cut a black hole in the listing,
/// which says "something is missing here" rather than "something is in front
/// of this" — so the test asserts a *darkened panel*, not an empty one.
#[test]
fn test_delete_confirm_drops_a_translucent_shadow() {
    let (terminal, _left, _right) = draw_delete_confirm(vec![PathBuf::from("/tmp/x.txt")]);
    let buf = terminal.backend().buffer();

    let panel_bg = crate::ui::FAR_BLUE;
    let ratatui::style::Color::Rgb(0, 0, panel_blue) = panel_bg else {
        panic!("the panel ground is a shade of pure blue");
    };
    let shadowed: Vec<&ratatui::buffer::Cell> = (0..SCREEN_H)
        .flat_map(|y| (0..SCREEN_W).map(move |x| (x, y)))
        .map(|(x, y)| &buf[(x, y)])
        .filter(|cell| {
            // The panel ground, scaled down: darker than the panel, and not
            // black either.
            matches!(cell.bg, ratatui::style::Color::Rgb(0, 0, b) if b > 0 && b < panel_blue)
        })
        .collect();

    assert!(
        !shadowed.is_empty(),
        "the dialog must darken the panel behind it"
    );
    assert!(
        shadowed.iter().any(|cell| cell.symbol().trim() != ""),
        "the shadow must let the panel's own characters show through"
    );
    // And the panel outside the shadow is untouched.
    let bright = (0..SCREEN_H)
        .flat_map(|y| (0..SCREEN_W).map(move |x| (x, y)))
        .filter(|&(x, y)| buf[(x, y)].bg == panel_bg)
        .count();
    assert!(bright > 0, "the rest of the panel keeps its own colour");
}
