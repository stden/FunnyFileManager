//! Purpose: the pairing of equal windows reads the clock once per
//! `CLOCK_EVERY` pairs and stops at the first reading past the deadline.

use std::time::{Duration, Instant};

use super::{scan_places, windows_by_hash, Clock, CLOCK_EVERY};
use crate::duplicates::lines::{kept_lines, Line};

const WIDTH: usize = 6;

/// One text of forty identical lines: thirty-five windows with one hash, which
/// pair 595 ways.
fn one_run_of_identical_lines() -> Vec<Vec<Line>> {
    vec![kept_lines(&["same"; 40].join("\n"))]
}

fn all_places(files: &[Vec<Line>]) -> Vec<(usize, usize)> {
    let mut groups = windows_by_hash(files, WIDTH).into_values();
    let places = groups.next().expect("the windows share one hash");
    assert!(groups.next().is_none());
    places
}

#[test]
fn the_clock_reads_the_time_at_every_256th_pair_only() {
    let mut clock = Clock {
        deadline: Instant::now(),
        pairs: 0,
    };
    for pair in 1..CLOCK_EVERY {
        assert!(!clock.expired(), "pair {pair} is not a clock reading");
    }
    assert!(
        clock.expired(),
        "pair {CLOCK_EVERY} reads the passed deadline"
    );
}

#[test]
fn a_clock_with_time_left_never_expires() {
    let mut clock = Clock {
        deadline: Instant::now() + Duration::from_secs(3600),
        pairs: 0,
    };
    assert!((0..1000).all(|_| !clock.expired()));
}

#[test]
fn pairing_stops_at_the_first_clock_reading_past_the_deadline() {
    let files = one_run_of_identical_lines();
    let places = all_places(&files);
    assert_eq!(places.len(), 35);
    let mut clock = Clock {
        deadline: Instant::now(),
        pairs: 0,
    };
    let mut found = Vec::new();
    assert!(!scan_places(&files, WIDTH, &places, &mut clock, &mut found));
    assert_eq!(
        clock.pairs, CLOCK_EVERY,
        "no pair after the stop is looked at"
    );
}

#[test]
fn pairing_with_time_left_looks_at_every_pair() {
    let files = one_run_of_identical_lines();
    let places = all_places(&files);
    let mut clock = Clock {
        deadline: Instant::now() + Duration::from_secs(3600),
        pairs: 0,
    };
    let mut found = Vec::new();
    assert!(scan_places(&files, WIDTH, &places, &mut clock, &mut found));
    assert_eq!(clock.pairs, 35 * 34 / 2);
    assert_eq!(
        found.len(),
        34,
        "only the pairs that start at the first window begin a block"
    );
}
