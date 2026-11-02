//! Purpose: copying from the vault — Enter and Ctrl+U put a secret on the clipboard and arm the wipe, OSC 52 as the fallback, and every failure said without arming it.

use super::*;

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
