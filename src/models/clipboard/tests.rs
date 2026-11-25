//! Session cloning and replacement copies must not retain clipboard ownership.

use super::*;

struct Backend;

impl ClipboardBackend for Backend {
    fn set_native(&mut self, _text: &str) -> Result<(), String> {
        Ok(())
    }
    fn emit_osc52(&mut self, _payload: &str) -> Result<(), String> {
        Ok(())
    }
}

#[test]
fn clone_drops_the_backend_and_deadline_while_text_copy_disarms_the_original() {
    let mut slot = ClipboardSlot::default();
    slot.install(Box::new(Backend));
    slot.clear_at = Some(Instant::now());
    let mut cloned = slot.clone();
    assert!(cloned.backend_mut().is_none());
    assert!(!cloned.is_armed());
    assert!(slot.backend_mut().is_some());
    assert!(slot.is_armed());
    slot.disarm();
    assert!(!slot.take_if_due(Instant::now()));
    assert!(!slot.is_armed());
}

#[test]
fn take_if_due_clears_expired_deadline_once() {
    let mut slot = ClipboardSlot::default();
    let now = Instant::now();
    slot.arm(now);
    assert!(slot.is_armed());
    assert!(!slot.take_if_due(now));
    assert!(slot.take_if_due(now + CLEAR_DELAY));
    assert!(!slot.is_armed());
    assert!(!slot.take_if_due(now + CLEAR_DELAY));
}
