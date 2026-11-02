//! Purpose: Progress remains observable while a job runs and cannot leak into the following job.
use super::*;
use crate::progress::{FileProgress, Operation, Phase, Tally, Transfer};
use std::time::Duration;

#[test]
fn transfer_reports_reach_the_current_job_without_an_output_stream() {
    let root = tempfile::TempDir::new().unwrap();
    let shared = Shared::default();
    let first = shared.reserve().unwrap();
    Reporter(shared.clone()).paint(&Snapshot {
        operation: Operation::Copy,
        phase: Phase::Working,
        current: root.path().join("source"),
        transfer: Some(Transfer {
            destination: root.path().join("destination"),
            file: FileProgress::Copying { done: 5, size: 10 },
        }),
        done: Tally { files: 0, bytes: 5 },
        total: Tally {
            files: 1,
            bytes: 10,
        },
        elapsed: Duration::from_secs(1),
    });
    let state = shared.snapshot().unwrap();
    assert_eq!(state["progress"]["done"]["bytes"], 5);
    assert_eq!(state["progress"]["transfer"]["permille"], 500);
    assert_eq!(state["job"]["id"], first.0);
    assert!(state["busy"].as_bool().unwrap());
    shared.complete(first, None).unwrap();
    shared.reserve().unwrap();
    assert!(shared.snapshot().unwrap()["progress"].is_null());
}
