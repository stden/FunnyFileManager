//! Unit tests for the vault-browser overlay's key handling.
//!
//! Everything goes through [`process_popup_input`] rather than the module's
//! own function, so the routing in `popup.rs` — the layer where a fall-through
//! bug would let a key reach the panels — is inside the test boundary too.

use std::cell::RefCell;
use std::fs::File;
use std::rc::Rc;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tempfile::TempDir;

use crate::clipboard::ClipboardBackend;
use crate::kdbx::tests_support::{
    password_for, unlocked_sample, username_for, vault_file, SAMPLE, VAULT_PASSWORD,
};
use crate::kdbx::KdbxSession;
use crate::models::{App, KdbxOverlay, Overlay, Panel};
use crate::popup::process_popup_input;

mod copy;
mod delete;
mod edit_form;
mod errors;
mod keymap;
mod structure;
mod unlock;

/// Records every native-route write where the test can read it back out; the
/// OSC 52 route always refuses, so a recorded value proves the native route
/// was taken and not merely preferred.
#[derive(Clone, Default)]
struct RecordingClipboard(Rc<RefCell<Vec<String>>>);

impl ClipboardBackend for RecordingClipboard {
    fn set_native(&mut self, text: &str) -> Result<(), String> {
        self.0.borrow_mut().push(text.to_owned());
        Ok(())
    }

    fn emit_osc52(&mut self, _payload: &str) -> Result<(), String> {
        Err("osc52 must not be reached when native works".to_owned())
    }
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}

fn ctrl(ch: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(ch), KeyModifiers::CONTROL)
}

/// Type `text` through the real key path, one character at a time.
fn type_str(app: &mut App, text: &str) {
    for ch in text.chars() {
        process_popup_input(app, key(KeyCode::Char(ch)));
    }
}

/// An `App` browsing the unlocked five-entry sample vault.
fn app_with_unlocked_vault() -> (TempDir, App) {
    let (dir, session, _now) = unlocked_sample();
    let mut app = App::new().expect("test app");
    app.set_overlay(Overlay::Kdbx(KdbxOverlay(session)));
    (dir, app)
}

/// The one match after filtering [`SAMPLE`] down to a single title — the
/// listing order is not stable, so tests navigate by filter, never by index.
fn focus(app: &mut App, title: &str) {
    type_str(app, title);
}

#[test]
fn navigation_keys_move_the_selection_and_backspace_edits_the_filter() {
    let (_dir, mut app) = app_with_unlocked_vault();

    process_popup_input(&mut app, key(KeyCode::Down));
    let after_down = app.kdbx_session().unwrap().vault().unwrap().selected();
    assert_eq!(after_down, 1);

    process_popup_input(&mut app, key(KeyCode::Up));
    assert_eq!(app.kdbx_session().unwrap().vault().unwrap().selected(), 0);

    // Page motions clamp to the ends of the five-entry list.
    process_popup_input(&mut app, key(KeyCode::PageDown));
    assert_eq!(app.kdbx_session().unwrap().vault().unwrap().selected(), 4);
    process_popup_input(&mut app, key(KeyCode::PageUp));
    assert_eq!(app.kdbx_session().unwrap().vault().unwrap().selected(), 0);

    type_str(&mut app, "Git");
    process_popup_input(&mut app, key(KeyCode::Backspace));
    assert_eq!(
        app.kdbx_session().unwrap().vault().unwrap().filter(),
        "Gi",
        "Backspace must edit the filter one character at a time"
    );
}

#[test]
fn helpers_do_nothing_when_no_vault_overlay_is_open() {
    // The guards exist so a stray call cannot panic; the routing layer never
    // takes these paths, so they are exercised directly.
    let mut app = App::new().expect("test app");
    let before = app.status_text();
    let now = std::time::Instant::now();

    super::unlock(&mut app, now);
    super::toggle_reveal(&mut app, now);
    super::copy(&mut app, crate::status::Secret::Password, now);

    assert!(matches!(app.overlay, Overlay::None));
    assert_eq!(app.status_text(), before, "guards must not report anything");
}

#[test]
fn probe_confirm_edit_esc_without_form() {
    let (_dir, mut session, now) = unlocked_sample();
    assert!(session.vault().expect("unlocked").form().is_none());
    super::form::confirm_edit_input(&mut session, KeyCode::Esc, now);
    assert!(session.vault().expect("unlocked").form().is_none());
}

#[test]
fn tmp_probe_begin_edit_without_a_vault_overlay_does_nothing() {
    let mut app = App::new().expect("test app");
    assert!(matches!(app.overlay, Overlay::None));
    super::begin_edit(&mut app, std::time::Instant::now());
    assert!(app.kdbx_session().is_none());
}
