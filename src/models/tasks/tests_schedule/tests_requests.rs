//! Purpose: unit tests for requests that stall or stop, a result nobody waited
//! for, a control's explicit sample, and an obsolete error.

use super::*;

#[test]
fn a_stalled_request_makes_a_sample_due_again() {
    let start = Instant::now();
    let mut refresh = Refresh::default();
    refresh.take_request(start);
    refresh.stalled();
    assert_eq!(refresh.schedule(), Schedule::Due);

    let mut idle = after_first_result(start);
    idle.stalled();
    assert_eq!(idle.schedule(), Schedule::At(start + REFRESH_INTERVAL));
}

/// A timer sample that stalls while the screen is held is not asked again:
/// its answer would have been discarded, so asking again would change a
/// held screen. One the user asked for is asked again, held or not.
#[test]
fn a_stalled_timer_sample_leaves_a_held_screen_held() {
    let start = Instant::now();
    let due = start + REFRESH_INTERVAL;
    let mut automatic = after_first_result(start);
    automatic.take_request(due);
    automatic.toggle_hold();
    automatic.stalled();
    assert_eq!(automatic.schedule(), Schedule::Held);
    assert_eq!(automatic.take_request(due + STALL_LIMIT), None);

    let mut explicit = after_first_result(start);
    explicit.take_request(due);
    explicit.toggle_hold();
    explicit.refresh_now();
    explicit.stalled();
    assert_eq!(explicit.schedule(), Schedule::Due);
}

#[test]
fn a_stopped_refresh_asks_for_nothing_ever_again() {
    let start = Instant::now();
    let mut refresh = after_first_result(start);
    refresh.stop();
    refresh.refresh_now();
    refresh.refresh_after_control();
    refresh.toggle_hold();
    refresh.toggle_hold();
    assert_eq!(refresh.schedule(), Schedule::Stopped);
    assert_eq!(refresh.take_request(start + STALL_LIMIT), None);
    assert_eq!(refresh.completed(start, REFRESH_INTERVAL), Arrival::Discard);
}

#[test]
fn a_result_nobody_waited_for_counts_as_automatic() {
    let start = Instant::now();
    let mut live = after_first_result(start);
    assert_eq!(live.completed(start, WARM_UP), Arrival::Apply);
    assert_eq!(live.schedule(), Schedule::At(start + WARM_UP));

    live.toggle_hold();
    assert_eq!(live.completed(start, WARM_UP), Arrival::Discard);
}

#[test]
fn a_control_without_a_read_in_flight_requests_one_explicit_sample() {
    let start = Instant::now();
    let timed = after_first_result(start);
    let mut held = timed;
    held.toggle_hold();
    for mut refresh in [Refresh::default(), timed, held] {
        let cadence = refresh.cadence();
        refresh.refresh_after_control();
        assert_eq!(refresh.schedule(), Schedule::Due);
        assert_eq!(refresh.take_request(start), Some(Request::Explicit));
        assert_eq!(refresh.completed(start, REFRESH_INTERVAL), Arrival::Apply);
        let expected = match cadence {
            Cadence::Live => Schedule::At(start + REFRESH_INTERVAL),
            Cadence::Held => Schedule::Held,
        };
        assert_eq!(refresh.schedule(), expected);
    }
}

#[test]
fn an_obsolete_error_is_discarded_but_the_followup_error_is_shown() {
    let now = Instant::now();
    let mut list = listing(vec![process(42, "firefox")]);
    list.refresh_now();
    assert_eq!(list.take_request(now), Some(Request::Explicit));
    list.refresh_after_control();
    list.apply_sample(Err(SampleError::Unsupported), now);
    assert_eq!(list.health(), Some(Health::Fresh));
    assert_eq!(shown(&list), [42]);
    assert_eq!(list.take_request(now), Some(Request::Explicit));
    list.apply_sample(Err(SampleError::Unsupported), now);
    assert_eq!(
        list.health(),
        Some(Health::Failed(SampleError::Unsupported))
    );
}
