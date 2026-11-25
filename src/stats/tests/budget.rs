//! Unit tests: a budget spends entries until they run out, then names the limit that stopped it.

use super::*;

#[test]
fn a_budget_spends_its_entries_and_then_names_them_as_the_stop() {
    let mut budget = Budget::new(2, None);
    assert!(budget.spend());
    assert!(budget.spend());
    assert_eq!(budget.stop(), None, "nothing refused yet");
    assert!(!budget.spend());
    assert_eq!(budget.stop(), Some(Stop::Entries));
}

#[test]
fn a_spent_budget_stays_spent() {
    let mut budget = Budget::new(0, None);
    assert!(!budget.spend());
    assert!(!budget.spend());
    assert_eq!(budget.stop(), Some(Stop::Entries));
}

#[test]
fn a_zero_time_budget_names_time_as_the_stop() {
    let mut budget = Budget::new(1_000, Some(Duration::ZERO));
    assert!(!budget.spend());
    assert_eq!(budget.stop(), Some(Stop::Time));
}

#[test]
fn a_generous_time_budget_never_stops_a_short_walk() {
    let mut budget = Budget::new(5, Some(Duration::from_secs(60)));
    for _ in 0..5 {
        assert!(budget.spend());
    }
    assert_eq!(budget.stop(), None);
}
