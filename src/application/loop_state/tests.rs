//! Purpose: a turn of the loop repaints when the clock locks an idle vault.

use super::*;
use crate::kdbx::tests_support::unlocked_sample;
use crate::models::{KdbxOverlay, Overlay};
use crate::wsl::SystemWslBackend;
use std::time::Duration;

#[test]
fn an_idle_vault_locked_by_the_clock_asks_for_a_frame() {
    let (_dir, mut session, _unlocked_at) = unlocked_sample();
    let idle_since = Instant::now()
        .checked_sub(crate::kdbx::IDLE_LOCK + Duration::from_secs(1))
        .expect("test setup");
    session.touch(idle_since);
    let mut app = App::new().expect("test app");
    app.set_overlay(Overlay::Kdbx(KdbxOverlay(session)));
    let mut state = LoopState::new(Arc::new(SystemWslBackend));
    state.redraw = Redraw::Unchanged;

    state.pump_background(&mut app);

    assert_eq!(state.redraw, Redraw::Needed);
    assert!(app.kdbx_session().expect("session").state.is_locked());
}

#[test]
fn a_vault_in_use_leaves_the_frame_alone() {
    let (_dir, session, _unlocked_at) = unlocked_sample();
    let mut app = App::new().expect("test app");
    app.set_overlay(Overlay::Kdbx(KdbxOverlay(session)));
    let mut state = LoopState::new(Arc::new(SystemWslBackend));
    state.redraw = Redraw::Unchanged;

    state.pump_background(&mut app);

    assert_eq!(state.redraw, Redraw::Unchanged);
    assert!(!app.kdbx_session().expect("session").state.is_locked());
}
