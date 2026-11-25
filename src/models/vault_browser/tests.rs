//! Unit tests for the `Clone`-safety wrappers around the vault session and
//! the clipboard slot.

use std::time::Instant;

use super::KdbxOverlay;
use crate::kdbx::tests_support::unlocked_sample;
use crate::models::ClipboardSlot;

#[test]
fn cloning_an_unlocked_overlay_yields_the_same_vault_locked() {
    let (_dir, session, _now) = unlocked_sample();
    let path = session.path.clone();
    let overlay = KdbxOverlay(session);

    let cloned = overlay.clone();

    assert!(cloned.0.state.is_locked(), "a clone must not carry secrets");
    assert_eq!(cloned.0.path, path, "a clone must point at the same file");
    assert!(
        !overlay.0.state.is_locked(),
        "cloning must not lock the original"
    );
}

#[test]
fn take_if_due_fires_once_at_the_deadline_and_disarms() {
    let mut slot = ClipboardSlot::default();
    let now = Instant::now();

    assert!(!slot.take_if_due(now), "nothing armed, nothing due");

    slot.arm(now);
    assert!(
        !slot.take_if_due(now + crate::CLIPBOARD_CLEAR / 2),
        "before the deadline the wipe must wait"
    );
    assert!(slot.take_if_due(now + crate::CLIPBOARD_CLEAR));
    assert!(
        !slot.is_armed(),
        "firing must disarm so the wipe stays one-shot"
    );
    assert!(
        !slot.take_if_due(now + crate::CLIPBOARD_CLEAR),
        "a fired deadline must not fire again"
    );
}
