//! Purpose: Keep bulk pulls useful after an individual failure or cancellation.

use super::super::fixtures::Fixture;
use super::*;

#[test]
fn a_failed_repository_does_not_prevent_later_repositories_from_updating() {
    let fixture = Fixture::new();
    let first = fixture.clone_repo("a-dirty");
    let second = fixture.clone_repo("nested/b-clean");
    std::fs::write(first.join("data.txt"), "keep local changes").expect("local change");
    fixture.advance();
    let mut report = Vec::new();
    let summary = execute(
        &fixture.root,
        |line| {
            report.push(line.to_owned());
            Ok(())
        },
        &|| false,
    )
    .expect("run");
    assert_eq!(summary.failed, 1);
    assert_eq!(summary.updated, 1);
    assert_eq!(
        std::fs::read_to_string(second.join("data.txt")).expect("updated file"),
        "remote update\n"
    );
    let report = report.join("\n");
    assert!(report.contains("a-dirty: Failed"));
    assert!(report.contains("b-clean: Updated"));
}

#[test]
fn no_repositories_and_an_unwritable_report_have_visible_results() {
    let temp = tempfile::TempDir::new().expect("temporary directory");
    let mut report = String::new();
    let summary = execute(
        temp.path(),
        |line| {
            report.push_str(line);
            Ok(())
        },
        &|| false,
    )
    .expect("empty directory");
    assert_eq!(summary.updated + summary.current + summary.failed, 0);
    assert!(report.contains("No Git repositories found"));
    let error = execute(
        temp.path(),
        |_| Err(io::Error::other("report unavailable")),
        &|| false,
    )
    .err()
    .expect("error");
    assert!(error.to_string().contains("report unavailable"));
}

#[test]
fn cancellation_stops_before_the_next_repository_changes() {
    let fixture = Fixture::new();
    fixture.clone_repo("a-first");
    let second = fixture.clone_repo("b-second");
    fixture.advance();
    let cancelled = std::cell::Cell::new(false);
    let summary = execute(
        &fixture.root,
        |line| {
            if line.starts_with("a-first: Updated") {
                cancelled.set(true);
            }
            Ok(())
        },
        &|| cancelled.get(),
    )
    .expect("cancelled run");
    assert!(summary.cancelled);
    assert_eq!(summary.updated, 1);
    assert_eq!(
        std::fs::read_to_string(second.join("data.txt")).expect("unchanged file"),
        "original\n"
    );
}
