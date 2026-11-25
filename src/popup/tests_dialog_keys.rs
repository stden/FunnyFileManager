//! Unit tests: the dialog keys a user finds when they do not type straight
//! into the field.
//!
//! Every modal here answers Tab and the arrows as well as Enter, and those
//! arms are the ones nothing exercises: a test that types a name and presses
//! Enter never touches them, while a person who reaches for Tab meets them
//! immediately. An arm that does the wrong thing there is worse than a missing
//! one, because the dialog looks like it is working.

use std::fs;

use crossterm::event::KeyCode;
use tempfile::TempDir;

use super::process_popup_input;
use super::tests::key;
use crate::copy::{CopyField, CopyPrompt, ExistingFiles, Purpose};
use crate::menu::{Heading, Menu};
use crate::models::{
    App, Confirm, Lifecycle, MaskPrompt, MaskPurpose, NameField, NamePrompt, Overlay, Panel,
    PanelSide,
};

mod tests_copy_dialog;
mod tests_guards;
mod tests_make_folder;
mod tests_mask_dialog;
mod tests_menu_bar;

fn app_over(names: &[&str]) -> (App, TempDir) {
    let dir = TempDir::new().expect("test setup");
    for name in names {
        fs::write(dir.path().join(name), b"x").expect("test setup");
    }
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(dir.path().to_path_buf()).expect("test setup");
    (app, dir)
}

fn press(app: &mut App, code: KeyCode) {
    process_popup_input(app, key(code));
}

// ─── the information panel ──────────────────────────────────────────────────

/// Ctrl+L put it up, so Ctrl+L's own letter takes it down again — along with
/// the three keys that dismiss every other panel.
#[test]
fn the_information_panel_closes_on_any_of_its_four_keys() {
    for code in [
        KeyCode::Esc,
        KeyCode::Enter,
        KeyCode::F(10),
        KeyCode::Char('l'),
        KeyCode::Char('L'),
    ] {
        let (mut app, _dir) = app_over(&[]);
        app.set_overlay(Overlay::Information(Box::default()));
        press(&mut app, code);
        assert!(!app.overlay.is_active(), "{code:?} should close it");
    }
}

#[test]
fn the_information_panel_ignores_everything_else() {
    let (mut app, _dir) = app_over(&[]);
    app.set_overlay(Overlay::Information(Box::default()));
    press(&mut app, KeyCode::Char('q'));
    assert!(matches!(app.overlay, Overlay::Information(_)));
}

// ─── the quit dialog ────────────────────────────────────────────────────────

/// Two answers, so Left, Right and Tab all mean "the other one".
#[test]
fn every_movement_key_in_the_quit_dialog_means_the_other_button() {
    for code in [KeyCode::Left, KeyCode::Right, KeyCode::Tab] {
        let (mut app, _dir) = app_over(&[]);
        app.set_overlay(Overlay::QuitConfirm(Confirm::Yes));
        press(&mut app, code);
        let Overlay::QuitConfirm(answer) = app.overlay else {
            panic!("still open");
        };
        assert!(!answer.is_yes(), "{code:?} moved to No");
        assert!(app.lifecycle.is_running(), "moving is not answering");
    }
}

#[test]
fn enter_on_no_closes_the_quit_dialog_without_quitting() {
    let (mut app, _dir) = app_over(&[]);
    app.set_overlay(Overlay::QuitConfirm(Confirm::No));
    press(&mut app, KeyCode::Enter);
    assert!(!app.overlay.is_active());
    assert!(app.lifecycle.is_running(), "No means no");
}

#[test]
fn the_quit_dialog_ignores_a_key_that_is_not_an_answer() {
    let (mut app, _dir) = app_over(&[]);
    app.set_overlay(Overlay::QuitConfirm(Confirm::Yes));
    press(&mut app, KeyCode::Char('z'));
    assert!(matches!(app.overlay, Overlay::QuitConfirm(Confirm::Yes)));
}

/// `y`/`Y` and `F10` all answer the question directly, without needing the
/// cursor to be on Yes first — the same shortcut Far's own dialog offers.
#[test]
fn y_or_f10_quits_from_the_quit_dialog_regardless_of_focus() {
    for code in [KeyCode::Char('y'), KeyCode::Char('Y'), KeyCode::F(10)] {
        let (mut app, _dir) = app_over(&[]);
        app.set_overlay(Overlay::QuitConfirm(Confirm::No));
        press(&mut app, code);
        assert!(!app.lifecycle.is_running(), "{code:?} should quit");
    }
}

/// `n`/`N` and `Esc` all cancel directly, without needing the cursor to be on
/// No first.
#[test]
fn n_or_esc_cancels_the_quit_dialog_regardless_of_focus() {
    for code in [KeyCode::Char('n'), KeyCode::Char('N'), KeyCode::Esc] {
        let (mut app, _dir) = app_over(&[]);
        app.set_overlay(Overlay::QuitConfirm(Confirm::Yes));
        press(&mut app, code);
        assert!(!app.overlay.is_active(), "{code:?} should close it");
        assert!(app.lifecycle.is_running(), "{code:?} must not quit");
    }
}

// ─── the name dialog walks backwards too ────────────────────────────────────

#[test]
fn shift_tab_and_up_walk_the_make_folder_controls_backwards() {
    let (mut app, _dir) = app_over(&[]);
    app.set_overlay(Overlay::NamePrompt(NamePrompt::make_folder()));

    for code in [KeyCode::BackTab, KeyCode::Up] {
        let before = current_field(&app);
        press(&mut app, code);
        assert_ne!(current_field(&app), before, "{code:?} moved the cursor");
    }
    // Backwards from the name field is the last control, not the first.
    let (mut app, _dir) = app_over(&[]);
    app.set_overlay(Overlay::NamePrompt(NamePrompt::make_folder()));
    press(&mut app, KeyCode::BackTab);
    assert_eq!(current_field(&app), NameField::Cancel);
}

#[test]
fn left_steps_the_link_drop_down_backwards() {
    let (mut app, _dir) = app_over(&[]);
    app.set_overlay(Overlay::NamePrompt(NamePrompt::make_folder()));
    press(&mut app, KeyCode::Down);
    assert_eq!(current_field(&app), NameField::Link);

    press(&mut app, KeyCode::Left);
    let Overlay::NamePrompt(prompt) = &app.overlay else {
        panic!("still open");
    };
    assert_eq!(prompt.link(), crate::models::LinkType::Symbolic);
}

fn current_field(app: &App) -> NameField {
    let Overlay::NamePrompt(prompt) = &app.overlay else {
        panic!("the name dialog is open");
    };
    prompt.field()
}
