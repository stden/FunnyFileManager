//! Purpose: Pins where the reader looks at the transcript: scrolling back and
//! forward, the jumps to either end, and the snap back on new output.

use super::*;

#[test]
fn the_view_starts_at_the_newest_lines() {
    let log = numbered(10);
    assert_eq!(log.scroll().lines(), 0);
    assert_eq!(shown(&log, 3), ["line 7", "line 8", "line 9"]);
}

#[test]
fn scrolling_up_walks_back_in_time_and_stops_at_the_oldest() {
    let mut log = numbered(10);
    log.scroll_up(2);
    assert_eq!(shown(&log, 3), ["line 5", "line 6", "line 7"]);

    // Holding the key must not run off the end of the ring.
    log.scroll_up(1_000);
    assert_eq!(log.scroll().lines(), 9);
    assert_eq!(shown(&log, 3), ["line 0"]);
}

#[test]
fn scrolling_down_returns_to_the_newest_and_stops_there() {
    let mut log = numbered(10);
    log.scroll_up(5);
    log.scroll_down(2);
    assert_eq!(shown(&log, 2), ["line 5", "line 6"]);

    log.scroll_down(1_000);
    assert_eq!(log.scroll().lines(), 0);
    assert_eq!(shown(&log, 2), ["line 8", "line 9"]);
}

#[test]
fn home_and_end_jump_to_the_ends() {
    let mut log = numbered(10);
    log.scroll_to_top();
    assert_eq!(shown(&log, 2), ["line 0"]);
    log.scroll_to_bottom();
    assert_eq!(shown(&log, 2), ["line 8", "line 9"]);
}

#[test]
fn new_output_snaps_the_view_back_to_the_bottom() {
    // A terminal does this, and it is what makes Ctrl+O after a command show
    // that command instead of wherever the reader left the scrollback.
    let mut log = numbered(10);
    log.scroll_up(5);
    log.record_output("fresh");
    assert_eq!(log.scroll().lines(), 0);
    assert_eq!(shown(&log, 2), ["line 9", "fresh"]);
}

#[test]
fn a_viewport_taller_than_the_log_shows_everything() {
    let log = numbered(3);
    assert_eq!(shown(&log, 50), ["line 0", "line 1", "line 2"]);
}

#[test]
fn scrolling_an_empty_log_is_harmless() {
    let mut log = ConsoleLog::default();
    log.scroll_up(5);
    log.scroll_down(5);
    log.scroll_to_top();
    assert_eq!(log.scroll().lines(), 0);
    assert_eq!(shown(&log, 10).len(), 0);
}

#[test]
fn visible_survives_absurd_arguments() {
    let mut log = ConsoleLog::default();
    assert_eq!(log.visible(0).len(), 0);
    log.record_output("a\nb\nc");
    assert_eq!(log.visible(0).len(), 0);
    log.scroll_up(usize::MAX);
    assert_eq!(shown(&log, usize::MAX), ["a"]);
    log.scroll_down(usize::MAX);
    assert_eq!(shown(&log, 1), ["c"]);
}

#[test]
fn scroll_back_is_a_distinct_offset_from_a_history_ring_index() {
    // Both count backwards, in different rings; the newtypes are what stop one
    // being handed to the other.
    let mut log = numbered(5);
    assert_eq!(log.scroll(), ScrollBack::BOTTOM);
    log.scroll_up(2);
    assert_eq!(log.scroll().lines(), 2);
    log.scroll_to_bottom();
    assert_eq!(log.scroll(), ScrollBack::BOTTOM);
}
