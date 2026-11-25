//! Unit tests: what the Wipe dialog prints — its title, its question, the
//! names it is about, its two buttons, and the red ground every question that
//! can lose data is drawn on.

use std::path::PathBuf;

use ratatui::Terminal;
use tempfile::TempDir;

use crate::models::{App, Overlay, Panel};
use crate::test_support::{buffer_text, WritableTestBackend};
/// Far's warning ground, so a test can tell the dialog from the panels behind
/// it without knowing where it landed.
use crate::ui::palette::WARNING_BG;
use crate::ui::ui;

const SCREEN_W: u16 = 80;
const SCREEN_H: u16 = 40;

/// The dialog drawn over panels listing empty directories, so no file name
/// from the working directory can satisfy an assertion by accident.
fn draw(paths: Vec<PathBuf>) -> (Terminal<WritableTestBackend>, TempDir) {
    let dir = TempDir::new().expect("temp dir");
    let left = Panel::new(dir.path().to_path_buf()).expect("left panel");
    let right = Panel::new(dir.path().to_path_buf()).expect("right panel");
    let mut app = App::with_panels(left, right);
    app.set_overlay(Overlay::WipeConfirm(paths));
    let mut terminal =
        Terminal::new(WritableTestBackend::new(SCREEN_W, SCREEN_H)).expect("test terminal");
    terminal.draw(|f| ui(f, &mut app)).expect("draw");
    assert!(app.dialog_mouse.area.is_some(), "the frame was tracked");
    (terminal, dir)
}

#[test]
fn names_the_single_file_it_is_about() {
    let (terminal, _dir) = draw(vec![PathBuf::from("/tmp/passwords.kdbx")]);
    let screen = buffer_text(&terminal);

    assert!(screen.contains(" Wipe "), "dialog title:\n{screen}");
    assert!(
        screen.contains("Do you wish to wipe the file?"),
        "dialog question:\n{screen}"
    );
    assert!(screen.contains("passwords.kdbx"), "file name:\n{screen}");
    assert!(
        !screen.contains("Coming in this release"),
        "no placeholder:\n{screen}"
    );
}

#[test]
fn counts_the_items_and_lists_their_names() {
    let (terminal, _dir) = draw(vec![
        PathBuf::from("/tmp/alpha.txt"),
        PathBuf::from("/tmp/beta.txt"),
        PathBuf::from("/tmp/gamma.txt"),
    ]);
    let screen = buffer_text(&terminal);

    assert!(
        screen.contains("Do you wish to wipe 3 items?"),
        "dialog question:\n{screen}"
    );
    for name in ["alpha.txt", "beta.txt", "gamma.txt"] {
        assert!(screen.contains(name), "{name} missing:\n{screen}");
    }
}

#[test]
fn falls_back_to_the_full_path_without_a_file_name() {
    let (terminal, _dir) = draw(vec![PathBuf::from("/tmp/project/..")]);
    let screen = buffer_text(&terminal);

    assert!(screen.contains("/tmp/project/.."), "{screen}");
}

#[test]
fn summarizes_the_files_past_the_first_five() {
    let paths: Vec<PathBuf> = (0..8)
        .map(|i| PathBuf::from(format!("/tmp/f{i}.txt")))
        .collect();
    let (terminal, _dir) = draw(paths);
    let screen = buffer_text(&terminal);

    assert!(screen.contains("f4.txt"), "fifth name:\n{screen}");
    assert!(!screen.contains("f5.txt"), "sixth is summarised:\n{screen}");
    assert!(screen.contains("… and 3 more"), "summary row:\n{screen}");
}

#[test]
fn offers_a_default_wipe_and_a_cancel_on_the_warning_ground() {
    let (terminal, _dir) = draw(vec![PathBuf::from("/tmp/x.txt")]);
    let screen = buffer_text(&terminal);
    assert!(screen.contains("{ Wipe }"), "default button:\n{screen}");
    assert!(screen.contains("[ Cancel ]"), "cancel button:\n{screen}");

    let buf = terminal.backend().buffer();
    let red = (0..SCREEN_H)
        .flat_map(|y| (0..SCREEN_W).map(move |x| (x, y)))
        .filter(|&(x, y)| buf[(x, y)].bg == WARNING_BG)
        .count();
    assert!(red > 0, "the wipe dialog must be drawn on red");
}
