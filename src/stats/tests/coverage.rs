//! Purpose: Whether the numbers are totals — an unreadable folder, an unlisted
//! one inside, and a walk that runs out of budget each say so.

use super::*;
use crate::stats::walk::{coverage, summarize, walk, Tally};

/// A folder that cannot be read still answers rather than failing — but it
/// says it could not be read. Its zeros are not an empty folder's, and
/// calling them complete is what showed an unreadable folder as "0 files".
#[test]
fn a_folder_that_cannot_be_read_is_not_reported_as_complete() {
    let stats = analyse(Path::new("/definitely/not/here"));
    assert_eq!(stats.files, 0);
    assert!(!stats.coverage.is_complete(), "{:?}", stats.coverage);
    assert!(
        matches!(&stats.coverage, Coverage::Unreadable(reason) if !reason.is_empty()),
        "{:?}",
        stats.coverage
    );
}

/// A folder inside that cannot be listed is counted as unread, and one is
/// enough to make every total a lower bound. Driven through `walk` itself:
/// as root no mode bits deny a listing, so only a path that is gone can.
#[test]
fn an_unlisted_folder_inside_makes_the_totals_lower_bounds() {
    let mut tally = Tally::default();
    let mut budget = 10;
    walk(
        Path::new("/definitely/not/here"),
        "gone",
        &mut tally,
        &mut budget,
    );
    assert_eq!(tally.unread, 1);

    let mut total = Tally::default();
    total.absorb(tally);
    assert_eq!(
        summarize(total, Vec::new(), budget).coverage,
        Coverage::Unread(1)
    );
    assert_eq!(
        coverage(0, 1),
        Coverage::Partial,
        "the walk limit is said first"
    );
    assert_eq!(coverage(5, 0), Coverage::Complete);
}

/// The same through `analyse`, with a real denied folder — skipped when
/// running as root, where the mode bits deny nothing.
#[cfg(unix)]
#[test]
fn a_denied_folder_inside_is_reported_and_the_rest_still_counted() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tree(&[("open/a.txt", 5), ("locked/hidden.txt", 7)]);
    let locked = dir.path().join("locked");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).expect("chmod");
    let readable = fs::read_dir(&locked).is_ok();
    let stats = analyse(dir.path());
    let locked_alone = analyse(&locked);
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).expect("chmod back");
    if readable {
        return;
    }
    assert_eq!(stats.files, 1);
    assert_eq!(stats.coverage, Coverage::Unread(1));
    assert!(
        matches!(&locked_alone.coverage, Coverage::Unreadable(reason) if reason.contains("denied")),
        "{:?}",
        locked_alone.coverage
    );
}

/// The walk stops at its budget and says so, because a number that is really a
/// lower bound must not be printed as if it were the whole answer.
#[test]
fn a_walk_that_runs_out_of_budget_reports_the_numbers_as_partial() {
    use super::{analyse_within, Coverage};

    let dir = TempDir::new().expect("test setup");
    for index in 0..5 {
        fs::write(dir.path().join(format!("f{index}.txt")), b"x").expect("test setup");
    }

    let complete = analyse_within(dir.path(), 100);
    assert_eq!(complete.coverage, Coverage::Complete);
    assert!(complete.coverage.is_complete());
    assert_eq!(complete.files, 5);

    let partial = analyse_within(dir.path(), 2);
    assert_eq!(partial.coverage, Coverage::Partial);
    assert!(!partial.coverage.is_complete());
    assert!(partial.files < 5, "it stopped early");
}

/// …and the budget is shared with the subdirectories, so a deep tree cannot
/// spend more than the whole walk was given.
#[test]
fn the_budget_is_shared_with_the_subtrees() {
    use super::{analyse_within, Coverage};

    let dir = TempDir::new().expect("test setup");
    let nested = dir.path().join("a/b/c");
    fs::create_dir_all(&nested).expect("test setup");
    for index in 0..5 {
        fs::write(nested.join(format!("f{index}.txt")), b"x").expect("test setup");
    }

    let partial = analyse_within(dir.path(), 4);
    assert_eq!(partial.coverage, Coverage::Partial);
}
