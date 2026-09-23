//! Purpose: the event-loop tick locks an idle vault and wipes a copied secret once its time is up.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use super::maintain_secrets;
use crate::clipboard::ClipboardBackend;
use crate::kdbx::tests_support::unlocked_sample;
use crate::models::{App, KdbxOverlay, Overlay};

/// Records every native write, so a wipe shows up as an empty string.
#[derive(Clone, Default)]
struct Writes(Rc<RefCell<Vec<String>>>);

impl ClipboardBackend for Writes {
    fn set_native(&mut self, text: &str) -> Result<(), String> {
        self.0.borrow_mut().push(text.to_owned());
        Ok(())
    }

    fn emit_osc52(&mut self, _payload: &str) -> Result<(), String> {
        Err("no terminal in test".to_owned())
    }
}

#[test]
fn an_idle_vault_locks_once_and_the_status_says_so() {
    let (_dir, session, unlocked_at) = unlocked_sample();
    let mut app = App::new().expect("test app");
    app.set_overlay(Overlay::Kdbx(KdbxOverlay(session)));
    let idle = unlocked_at + crate::kdbx::IDLE_LOCK;
    let just_before = unlocked_at + crate::kdbx::IDLE_LOCK.saturating_sub(Duration::from_secs(1));

    assert!(!maintain_secrets(&mut app, just_before));
    assert!(!app.kdbx_session().expect("session").state.is_locked());

    assert!(maintain_secrets(&mut app, idle));
    assert!(app.kdbx_session().expect("session").state.is_locked());
    assert_eq!(app.status_text(), "Vault locked");

    assert!(
        !maintain_secrets(&mut app, idle + Duration::from_secs(1)),
        "an already locked vault is not reported again"
    );
}

#[test]
fn a_copied_secret_is_wiped_when_its_countdown_expires() {
    let mut app = App::new().expect("test app");
    let writes = Writes::default();
    app.clipboard.install(Box::new(writes.clone()));
    let copied_at = Instant::now();
    app.clipboard.arm(copied_at);
    let due = copied_at + crate::CLIPBOARD_CLEAR;
    let just_before = copied_at + crate::CLIPBOARD_CLEAR.saturating_sub(Duration::from_millis(1));

    maintain_secrets(&mut app, just_before);
    assert!(writes.0.borrow().is_empty(), "wiped before its time");
    assert!(app.clipboard.is_armed());

    assert!(!maintain_secrets(&mut app, due), "no vault was locked");
    assert_eq!(writes.0.borrow().as_slice(), [String::new()]);
    assert!(!app.clipboard.is_armed());

    maintain_secrets(&mut app, due + Duration::from_secs(60));
    assert_eq!(writes.0.borrow().len(), 1, "the wipe happens once");
}
