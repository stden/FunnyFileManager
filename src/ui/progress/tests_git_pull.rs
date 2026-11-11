//! Purpose: Explain bulk Git progress with repository counts and a cancel key.

use super::tests::{rows, screen};
use super::*;
use crate::progress::Tally;
use std::{path::PathBuf, time::Duration};

#[test]
fn git_pull_progress_names_repositories_and_cancellation() {
    let state = Snapshot {
        operation: Operation::GitPull,
        phase: Phase::Working,
        current: PathBuf::from("workspace/repo"),
        transfer: None,
        done: Tally { files: 1, bytes: 0 },
        total: Tally { files: 4, bytes: 0 },
        elapsed: Duration::from_secs(2),
    };
    let text = rows(&progress_dialog(screen(), &state)).join("\n");
    assert!(text.contains("Git pull"));
    assert!(text.contains("Repositories: 1 of 4"));
    assert!(text.contains("Esc / Ctrl+C: cancel"));
    let scanning = Snapshot {
        phase: Phase::Scanning,
        ..state
    };
    let text = rows(&progress_dialog(screen(), &scanning)).join("\n");
    assert!(text.contains("Repositories found: 4"));
}
