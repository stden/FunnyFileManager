//! Purpose: Unit tests for `ops::should_poll_cancel` throttle boundaries and `live_cancel`.

use std::time::{Duration, Instant};

use crate::ops::{live_cancel, should_poll_cancel, CANCEL_POLL_INTERVAL};
use crate::terminal::headless::HeadlessHost;

#[test]
fn should_poll_cancel_enforces_shown_flag_and_exact_25ms_interval() {
    let t0 = Instant::now();
    let mut last = None;

    assert!(!should_poll_cancel(false, &mut last, t0));
    assert_eq!(last, None);

    assert!(should_poll_cancel(true, &mut last, t0));
    assert_eq!(last, Some(t0));

    assert!(!should_poll_cancel(
        true,
        &mut last,
        t0 + Duration::from_millis(24)
    ));
    assert_eq!(last, Some(t0));

    let at_boundary = t0 + CANCEL_POLL_INTERVAL;
    assert!(should_poll_cancel(true, &mut last, at_boundary));
    assert_eq!(last, Some(at_boundary));

    assert!(!should_poll_cancel(
        true,
        &mut last,
        at_boundary + Duration::from_millis(24)
    ));
    assert!(should_poll_cancel(
        true,
        &mut last,
        at_boundary + CANCEL_POLL_INTERVAL
    ));
}

#[test]
fn live_cancel_throttles_immediate_repolls_against_headless_host() {
    let host = HeadlessHost::new();
    let _scope = host.enter();
    let mut poll = live_cancel();

    host.cancel();
    assert!(!poll(false));
    host.reset().expect("reset host");

    assert!(!poll(true));
    host.cancel();
    assert!(
        !poll(true),
        "immediate second poll within 25 ms is throttled"
    );
}
