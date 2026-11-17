//! Purpose: Session publication keeps one job, monotonic revisions and actionable errors.
use super::*;

#[test]
fn reservations_reject_busy_and_stopped_sessions() {
    let shared = Shared::default();
    shared.update(json!({"running": true})).unwrap();
    let first = shared.reserve().unwrap();
    assert_eq!(
        shared.reserve().err().unwrap().kind(),
        io::ErrorKind::WouldBlock
    );
    shared.progress(json!({"done": 1})).unwrap();
    shared.complete(first, Some("cancelled".into())).unwrap();
    let completed = shared.snapshot().unwrap();
    assert_eq!(completed["job"]["status"], "finished");
    assert_eq!(completed["job"]["error"], "cancelled");
    let second = shared.reserve().unwrap();
    assert!(second.0 > first.0);
    assert!(shared.snapshot().unwrap()["progress"].is_null());
    shared.stop(Some("worker stopped".into())).unwrap();
    let stopped = shared.snapshot().unwrap();
    assert_eq!(stopped["running"], false);
    assert_eq!(stopped["busy"], false);
    assert_eq!(stopped["job"]["error"], "worker stopped");
    assert_eq!(
        shared.reserve().err().unwrap().kind(),
        io::ErrorKind::BrokenPipe
    );
}

#[test]
fn identical_snapshots_do_not_increment_the_revision() {
    let shared = Shared::default();
    let application = json!({"running": true, "screen": "initial"});
    shared.update(application.clone()).unwrap();
    let first = shared.snapshot().unwrap();
    shared.update(application).unwrap();
    assert_eq!(first, shared.snapshot().unwrap());
    shared
        .update(json!({"running": true, "screen": "changed"}))
        .unwrap();
    assert!(
        shared.snapshot().unwrap()["revision"].as_u64().unwrap()
            > first["revision"].as_u64().unwrap()
    );
}

#[test]
fn poisoned_publication_returns_errors_instead_of_panicking() {
    let shared = poisoned();
    assert!(shared
        .snapshot()
        .unwrap_err()
        .to_string()
        .contains("session state lock failed"));
    assert!(shared.update(json!({"running": true})).is_err());
    assert!(shared.progress(json!({"done": 1})).is_err());
    assert!(shared.stop(None).is_err());
}

pub(in crate::automation::session) fn poisoned() -> Shared {
    let shared = Shared::default();
    let poisoned = shared.clone();
    let _panic = std::thread::spawn(move || {
        let _guard = poisoned.0.lock().unwrap();
        panic!("fixture poisons the publication lock");
    })
    .join();
    shared
}

#[test]
fn failed_progress_publication_does_not_abort_the_transfer() {
    use crate::progress::{Operation, Painter, Phase, Snapshot, Tally};
    let shared = poisoned();
    let mut reporter = super::super::progress::Reporter(shared.clone());
    reporter.paint(&Snapshot {
        operation: Operation::Copy,
        phase: Phase::Working,
        current: "source".into(),
        transfer: None,
        done: Tally::default(),
        total: Tally::default(),
        elapsed: std::time::Duration::ZERO,
    });
    assert!(shared.snapshot().is_err());
}
