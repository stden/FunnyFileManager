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

mod errors;
mod structure;

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
fn f4_edits_and_f2_saves_the_selected_entry_through_the_popup() {
    let (_dir, mut app) = app_with_unlocked_vault();
    focus(&mut app, "GitHub");
    process_popup_input(&mut app, key(KeyCode::F(4)));
    assert!(app
        .kdbx_session()
        .and_then(KdbxSession::vault)
        .and_then(crate::kdbx::Vault::form)
        .is_some());
    for _ in 0..4 {
        process_popup_input(&mut app, key(KeyCode::Tab));
    }
    type_str(&mut app, "-rotated");
    process_popup_input(&mut app, key(KeyCode::F(2)));
    let session = app.kdbx_session_mut().expect("session");
    assert!(session.vault().expect("vault").form().is_none());
    assert!(session
        .copy_password(std::time::Instant::now())
        .expect("password")
        .ends_with("-rotated"));
}

#[test]
fn kdbx_overlay_swallows_f8_without_touching_the_panel() {
    let (_vault_dir, mut app) = app_with_unlocked_vault();
    let panel_dir = TempDir::new().expect("temp dir");
    File::create(panel_dir.path().join("precious.txt")).expect("create file");
    app.left_panel = Panel::new(panel_dir.path().to_path_buf()).expect("panel");
    app.left_panel.select(Some(1));

    let consumed = process_popup_input(&mut app, key(KeyCode::F(8)));

    assert!(consumed);
    assert!(app.kdbx_session().is_some(), "the overlay must stay up");
    assert!(matches!(
        app.kdbx_session().unwrap().vault().unwrap().page,
        crate::kdbx::vault::Page::DeleteConfirm(_)
    ));
    assert!(
        panel_dir.path().join("precious.txt").exists(),
        "F8 must not reach the delete handler"
    );
}

#[test]
fn f8_deletes_the_selected_entry_and_ctrl_z_restores_it() {
    let (_dir, mut app) = app_with_unlocked_vault();
    focus(&mut app, "GitHub");

    process_popup_input(&mut app, key(KeyCode::F(8)));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert_eq!(
        app.kdbx_session().unwrap().vault().unwrap().entries().len(),
        4
    );
    assert!(app.status_text().contains("KeePass entry deleted"));

    process_popup_input(&mut app, ctrl('z'));
    assert_eq!(
        app.kdbx_session().unwrap().vault().unwrap().entries().len(),
        5
    );
    assert!(app.status_text().contains("undone"));
}

#[test]
fn ctrl_c_copies_the_visible_password_from_the_edit_form() {
    let (_dir, mut app) = app_with_unlocked_vault();
    let recorder = RecordingClipboard::default();
    app.clipboard.install(Box::new(recorder.clone()));
    focus(&mut app, "GitHub");
    process_popup_input(&mut app, key(KeyCode::F(4)));

    process_popup_input(&mut app, ctrl('c'));

    assert_eq!(recorder.0.borrow().as_slice(), [password_for("GitHub")]);
    assert!(app
        .kdbx_session()
        .unwrap()
        .vault()
        .unwrap()
        .form()
        .is_some());
}

#[test]
fn ctrl_z_undoes_the_latest_edit_keystroke() {
    let (_dir, mut app) = app_with_unlocked_vault();
    focus(&mut app, "GitHub");
    process_popup_input(&mut app, key(KeyCode::F(4)));
    type_str(&mut app, "X");

    process_popup_input(&mut app, ctrl('z'));

    assert_eq!(
        app.kdbx_session()
            .unwrap()
            .vault()
            .unwrap()
            .form()
            .unwrap()
            .title(),
        "GitHub"
    );
}

#[test]
fn dirty_form_asks_to_save_on_exit_and_n_discards() {
    let (_dir, mut app) = app_with_unlocked_vault();
    focus(&mut app, "GitHub");
    process_popup_input(&mut app, key(KeyCode::F(4)));
    type_str(&mut app, "X");

    process_popup_input(&mut app, key(KeyCode::Esc));
    assert_eq!(
        app.kdbx_session()
            .unwrap()
            .vault()
            .unwrap()
            .form()
            .unwrap()
            .mode(),
        crate::kdbx::FormMode::ConfirmSave
    );
    process_popup_input(&mut app, key(KeyCode::Char('n')));
    assert!(app
        .kdbx_session()
        .unwrap()
        .vault()
        .unwrap()
        .form()
        .is_none());
}

#[test]
fn enter_saves_when_a_dirty_form_asks_on_exit() {
    let (_dir, mut app) = app_with_unlocked_vault();
    focus(&mut app, "GitHub");
    process_popup_input(&mut app, key(KeyCode::F(4)));
    type_str(&mut app, "X");
    process_popup_input(&mut app, key(KeyCode::Esc));

    process_popup_input(&mut app, key(KeyCode::Enter));

    let vault = app.kdbx_session().unwrap().vault().unwrap();
    assert!(vault.form().is_none());
    assert!(vault.visible().any(|entry| entry.title == "GitHubX"));
}

#[test]
fn ctrl_r_toggles_the_typed_master_password() {
    let (_dir, path) = vault_file(SAMPLE, VAULT_PASSWORD, None);
    let mut app = App::new().expect("test app");
    app.set_overlay(Overlay::Kdbx(KdbxOverlay(KdbxSession::new(path))));
    type_str(&mut app, "secret");

    process_popup_input(&mut app, ctrl('r'));

    assert!(app.kdbx_session().unwrap().prompt().unwrap().is_revealed());
}

#[test]
fn enter_unlocks_with_the_right_password() {
    let (_dir, path) = vault_file(SAMPLE, VAULT_PASSWORD, None);
    let mut app = App::new().expect("test app");
    app.set_overlay(Overlay::Kdbx(KdbxOverlay(KdbxSession::new(path))));

    type_str(&mut app, VAULT_PASSWORD);
    process_popup_input(&mut app, key(KeyCode::Enter));

    assert!(app
        .kdbx_session()
        .is_some_and(|session| !session.state.is_locked()));
    assert!(
        app.status_text().contains("Vault unlocked: 5 entries"),
        "{}",
        app.status_text()
    );
}

#[test]
fn a_wrong_password_reports_failure_and_stays_locked() {
    let (_dir, path) = vault_file(SAMPLE, VAULT_PASSWORD, None);
    let mut app = App::new().expect("test app");
    app.set_overlay(Overlay::Kdbx(KdbxOverlay(KdbxSession::new(path))));

    type_str(&mut app, "not the password");
    process_popup_input(&mut app, key(KeyCode::Enter));

    let session = app.kdbx_session().expect("overlay still up");
    assert!(session.state.is_locked());
    // The status line carries no detail; the reason sits beside the field.
    assert!(
        app.status_text().contains("Unlock failed"),
        "{}",
        app.status_text()
    );
    assert!(session.prompt().expect("locked").error().is_some());
}

#[test]
fn enter_without_a_clipboard_backend_reports_it() {
    let (_dir, mut app) = app_with_unlocked_vault();

    process_popup_input(&mut app, key(KeyCode::Enter));

    assert!(
        app.status_text().contains("no clipboard available"),
        "{}",
        app.status_text()
    );
    assert!(
        !app.clipboard.is_armed(),
        "a failed copy must not arm the auto-clear"
    );
}

#[test]
fn enter_copies_the_selected_password_and_arms_the_wipe() {
    let (_dir, mut app) = app_with_unlocked_vault();
    let recorder = RecordingClipboard::default();
    app.clipboard.install(Box::new(recorder.clone()));
    focus(&mut app, "GitHub");

    process_popup_input(&mut app, key(KeyCode::Enter));

    assert_eq!(recorder.0.borrow().as_slice(), [password_for("GitHub")]);
    assert!(
        app.status_text().contains("Password copied"),
        "{}",
        app.status_text()
    );
    assert!(app.clipboard.is_armed());
}

#[test]
fn ctrl_u_copies_the_user_name() {
    let (_dir, mut app) = app_with_unlocked_vault();
    let recorder = RecordingClipboard::default();
    app.clipboard.install(Box::new(recorder.clone()));
    focus(&mut app, "GitHub");

    process_popup_input(&mut app, ctrl('u'));

    assert_eq!(recorder.0.borrow().as_slice(), [username_for("GitHub")]);
    assert!(
        app.status_text().contains("User name copied"),
        "{}",
        app.status_text()
    );
}

#[test]
fn ctrl_r_reveals_and_hides_again() {
    let (_dir, mut app) = app_with_unlocked_vault();
    focus(&mut app, "GitHub");

    process_popup_input(&mut app, ctrl('r'));
    assert!(app
        .kdbx_session()
        .and_then(|session| session.vault())
        .is_some_and(crate::kdbx::Vault::is_revealed));

    process_popup_input(&mut app, ctrl('r'));
    assert!(!app
        .kdbx_session()
        .and_then(|session| session.vault())
        .is_some_and(crate::kdbx::Vault::is_revealed));
}

#[test]
fn esc_remembers_and_closes_the_overlay() {
    let (_dir, mut app) = app_with_unlocked_vault();

    let consumed = process_popup_input(&mut app, key(KeyCode::Esc));

    assert!(consumed);
    assert!(matches!(app.overlay, Overlay::None));
    assert!(
        app.status_text().contains("key remembered"),
        "{}",
        app.status_text()
    );
}

#[test]
fn ctrl_c_still_quits_but_locks_on_the_way_out() {
    let (_dir, mut app) = app_with_unlocked_vault();

    process_popup_input(&mut app, ctrl('c'));

    assert!(!app.lifecycle.is_running());
    assert!(
        matches!(app.overlay, Overlay::None),
        "the session must be locked and dropped at the quit gesture"
    );
}

/// Backend whose native route refuses, forcing the OSC 52 fallback.
struct Osc52OnlyClipboard(Rc<RefCell<Vec<String>>>);

impl ClipboardBackend for Osc52OnlyClipboard {
    fn set_native(&mut self, _text: &str) -> Result<(), String> {
        Err("no display".to_owned())
    }

    fn emit_osc52(&mut self, payload: &str) -> Result<(), String> {
        self.0.borrow_mut().push(payload.to_owned());
        Ok(())
    }
}

/// Backend with no working route at all.
struct DeadClipboard;

impl ClipboardBackend for DeadClipboard {
    fn set_native(&mut self, _text: &str) -> Result<(), String> {
        Err("native down".to_owned())
    }

    fn emit_osc52(&mut self, _payload: &str) -> Result<(), String> {
        Err("osc52 down".to_owned())
    }
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
fn reveal_with_nothing_selected_reports_the_error() {
    let (_dir, mut app) = app_with_unlocked_vault();
    type_str(&mut app, "zzz-no-such-entry");

    process_popup_input(&mut app, ctrl('r'));

    assert!(
        app.status_text().contains("No entry selected"),
        "{}",
        app.status_text()
    );
}

#[test]
fn copy_with_nothing_selected_reports_the_failure() {
    let (_dir, mut app) = app_with_unlocked_vault();
    type_str(&mut app, "zzz-no-such-entry");

    process_popup_input(&mut app, key(KeyCode::Enter));

    assert!(
        app.status_text().contains("Copy failed: No entry selected"),
        "{}",
        app.status_text()
    );
}

#[test]
fn copy_falls_back_to_osc52_and_says_so() {
    let (_dir, mut app) = app_with_unlocked_vault();
    let sink = Rc::new(RefCell::new(Vec::new()));
    app.clipboard
        .install(Box::new(Osc52OnlyClipboard(Rc::clone(&sink))));
    focus(&mut app, "GitHub");

    process_popup_input(&mut app, key(KeyCode::Enter));

    assert!(
        app.status_text()
            .contains("sent to the terminal via OSC 52"),
        "{}",
        app.status_text()
    );
    let payloads = sink.borrow();
    assert_eq!(payloads.len(), 1);
    assert!(payloads[0].starts_with("\x1b]52;c;"));
    assert!(app.clipboard.is_armed());
}

#[test]
fn copy_with_no_working_route_reports_and_does_not_arm() {
    let (_dir, mut app) = app_with_unlocked_vault();
    app.clipboard.install(Box::new(DeadClipboard));
    focus(&mut app, "GitHub");

    process_popup_input(&mut app, key(KeyCode::Enter));

    assert!(
        app.status_text().contains("Copy failed"),
        "{}",
        app.status_text()
    );
    assert!(
        !app.clipboard.is_armed(),
        "a copy that reached no clipboard must not schedule a wipe"
    );
}

#[test]
fn zzz_probe_delete_cancel_sets_status() {
    let (_dir, mut app) = app_with_unlocked_vault();
    focus(&mut app, "GitHub");
    process_popup_input(&mut app, key(KeyCode::F(8)));
    assert!(matches!(
        app.kdbx_session().unwrap().vault().unwrap().page,
        crate::kdbx::vault::Page::DeleteConfirm(_)
    ));

    let consumed = process_popup_input(&mut app, key(KeyCode::Char('n')));

    assert!(consumed);
    assert!(matches!(
        app.kdbx_session().unwrap().vault().unwrap().page,
        crate::kdbx::vault::Page::Browse(_)
    ));
    assert_eq!(
        app.kdbx_session().unwrap().vault().unwrap().entries().len(),
        5
    );
    assert!(
        app.status_text().contains("Delete cancelled"),
        "status was {:?}",
        app.status_text()
    );
}

#[test]
fn zzz_probe_delete_cancel_via_esc_sets_status() {
    let (_dir, mut app) = app_with_unlocked_vault();
    focus(&mut app, "GitHub");
    process_popup_input(&mut app, key(KeyCode::F(8)));

    process_popup_input(&mut app, key(KeyCode::Esc));

    assert!(app.kdbx_session().is_some(), "overlay must stay up");
    assert!(app.status_text().contains("Delete cancelled"));
}

#[test]
fn probe_confirm_edit_esc_without_form() {
    let (_dir, mut session, now) = unlocked_sample();
    assert!(session.vault().expect("unlocked").form().is_none());
    super::confirm_edit_input(&mut session, KeyCode::Esc, now);
    assert!(session.vault().expect("unlocked").form().is_none());
}

#[test]
fn tmp_probe_begin_edit_without_a_vault_overlay_does_nothing() {
    let mut app = App::new().expect("test app");
    assert!(matches!(app.overlay, Overlay::None));
    super::begin_edit(&mut app, std::time::Instant::now());
    assert!(app.kdbx_session().is_none());
}
