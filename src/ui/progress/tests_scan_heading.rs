//! Purpose: A git pull still finding repositories names its phase and offers
//! cancel, as a pull that is already working does.

use std::path::PathBuf;
use std::time::Duration;

use super::*;
use crate::progress::Tally;

fn heading(operation: Operation, phase: Phase) -> String {
    let snapshot = Snapshot {
        operation,
        phase,
        current: PathBuf::from("/srv/x/repos"),
        transfer: None,
        done: Tally::default(),
        total: Tally::default(),
        elapsed: Duration::ZERO,
    };
    lines(&snapshot, 60)
        .first()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn a_git_pull_still_finding_repositories_says_so_and_offers_cancel() {
    assert_eq!(
        heading(Operation::GitPull, Phase::Scanning),
        "Finding repositories (Esc / Ctrl+C: cancel)"
    );
}
