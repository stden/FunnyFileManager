//! Purpose: Temp cleanup rendering has accurate preview details and a default-No warning.
use super::*;
use crate::models::temp_cleanup::{FileIdentity, PlannedTempFile, TempCutoff, TempError, TempRoot};
use crate::models::FileSize;
use ratatui::{backend::TestBackend, Terminal};
use std::{
    fs,
    path::Path,
    time::{Duration, SystemTime},
};
use tempfile::TempDir;

fn identity(path: &Path) -> FileIdentity {
    FileIdentity(file_id::get_file_id(path).unwrap())
}

fn old_file(path: &Path, now: SystemTime) -> SystemTime {
    fs::write(path, b"old").unwrap();
    let modified = now - Duration::from_hours(192);
    fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(modified)
        .unwrap();
    modified
}

fn plan() -> (TempDir, TempPlan) {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("old.tmp");
    let now = SystemTime::now();
    let modified = old_file(&path, now);
    let plan = TempPlan {
        root: TempRoot {
            path: temp.path().to_path_buf(),
            identity: identity(temp.path()),
        },
        cutoff: TempCutoff::at(now).unwrap(),
        files: vec![PlannedTempFile {
            identity: identity(&path),
            path,
            modified,
            size: FileSize::from_bytes(3),
        }],
        bytes: FileSize::from_bytes(3),
        skipped: 2,
    };
    (temp, plan)
}

fn display(state: &TempCleanupState) -> String {
    let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
    let mut tracker = crate::mouse::DialogMouse::default();
    terminal
        .draw(|frame| render_state(frame, &mut tracker, state))
        .unwrap();
    crate::test_support::buffer_string(terminal.backend().buffer())
}

#[test]
fn preview_lists_only_temp_policy_and_eligible_bytes() {
    let (_temp, plan) = plan();
    let text = display(&TempCleanupState::Preview {
        plan,
        focus: TempChoice::Cancel,
    });
    assert!(text.contains("older than 7 days"));
    assert!(text.contains("Eligible files: 1 (3 B)"));
    assert!(text.contains("Skipped during scan: 2"));
    assert!(text.contains("{ Cancel }"));
    assert!(!text.contains("Docker"));
    assert!(!text.contains("Cargo"));
}

#[test]
fn destructive_question_defaults_to_no() {
    let (_temp, plan) = plan();
    let text = display(&TempCleanupState::Confirm {
        plan,
        answer: Confirm::No,
    });
    assert!(text.contains("Permanently delete"));
    assert!(text.contains("{ No }"), "{text}");
    assert!(!text.contains("{ Yes }"));
}

#[test]
fn busy_result_and_failure_states_explain_the_outcome() {
    assert!(display(&TempCleanupState::Scanning).contains("Scanning temporary files"));
    let result = TempOutcome {
        removed: 2,
        skipped: 3,
        failures: 1,
        freed: FileSize::from_bytes(12),
    };
    let text = display(&TempCleanupState::Finished(result));
    assert!(text.contains("Removed 12 B in 2 temporary files"));
    assert!(text.contains("Skipped: 3; could not remove: 1"));
    assert!(display(&TempCleanupState::Failed(TempError::UnsafeRoot)).contains("cleanup refused"));
}
