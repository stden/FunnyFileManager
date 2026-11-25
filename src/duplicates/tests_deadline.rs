//! Purpose: the search stops at its deadline, and says it did not look everywhere.

use std::time::{Duration, Instant};

use super::find;

#[test]
fn a_search_past_its_deadline_is_incomplete_and_reports_nothing() {
    let block = (1..=6)
        .map(|n| format!("late line {n}"))
        .collect::<Vec<_>>()
        .join("\n");
    let search = find(&[block.as_str(), block.as_str()], 6, 10, Instant::now());
    assert!(!search.complete);
    assert_eq!(search.groups.len(), 0);
}

#[test]
fn a_long_run_of_one_line_stops_at_its_deadline() {
    let run = std::iter::repeat_n("same", 200_000)
        .collect::<Vec<_>>()
        .join("\n");
    let started = Instant::now();
    let search = find(&[run.as_str()], 6, 10, started + Duration::from_millis(50));
    assert!(
        !search.complete,
        "200 thousand lines cannot be compared in 50 ms"
    );
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "{:?}",
        started.elapsed()
    );
}
