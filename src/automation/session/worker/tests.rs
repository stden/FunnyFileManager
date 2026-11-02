//! Purpose: Engine startup and shutdown preserve an observable, recoverable session state.
use super::*;
use crate::automation::{catalog, options::Profile};
use ratatui::layout::Size;
use std::sync::mpsc;

fn options(root: &std::path::Path) -> Options {
    Options {
        root: root.to_owned(),
        size: Size::new(80, 24),
        profile: Profile::Isolated,
    }
}

#[test]
fn abandoned_startup_does_not_leave_a_live_runtime() {
    let directory = tempfile::TempDir::new().unwrap();
    let shared = Shared::default();
    let (sender, receiver) = mpsc::sync_channel(1);
    drop(receiver);
    assert!(initialize(&options(directory.path()), &shared, &sender).is_none());
    assert_eq!(shared.snapshot().unwrap()["busy"], false);
}

#[test]
fn disconnected_transport_stops_the_application_snapshot() {
    let directory = tempfile::TempDir::new().unwrap();
    let host = HeadlessHost::new();
    let _scope = host.enter();
    let mut runtime = Runtime::new(directory.path(), Size::new(80, 24), Profile::Isolated).unwrap();
    let shared = Shared::default();
    let (sender, receiver) = mpsc::sync_channel(1);
    drop(sender);
    drive(
        &mut runtime,
        &receiver,
        &shared,
        &host,
        &AtomicBool::new(false),
    );
    assert_eq!(shared.snapshot().unwrap()["running"], false);
}

#[test]
fn idle_refresh_publishes_a_changed_pending_state() {
    let directory = tempfile::TempDir::new().unwrap();
    let mut runtime = Runtime::new(directory.path(), Size::new(80, 24), Profile::Isolated).unwrap();
    let shared = Shared::default();
    assert!(refresh_idle(&mut runtime, &shared, true));
    assert_eq!(shared.snapshot().unwrap()["background_pending"], false);
    assert_eq!(shared.snapshot().unwrap()["screen"]["width"], 80);
}

#[test]
fn wrong_context_finishes_the_job_with_a_visible_error() {
    let directory = tempfile::TempDir::new().unwrap();
    let mut runtime = Runtime::new(directory.path(), Size::new(80, 24), Profile::Isolated).unwrap();
    let host = HeadlessHost::new();
    let _scope = host.enter();
    let shared = Shared::default();
    let id = shared.reserve().unwrap();
    let invocation = catalog::resolve("editor", "cursor_down").unwrap();
    execute(
        &mut runtime,
        &shared,
        &host,
        Command {
            id,
            operation: Operation::Action(invocation),
        },
    );
    let state = shared.snapshot().unwrap();
    assert_eq!(state["busy"], false);
    assert_eq!(state["job"]["status"], "finished");
    assert!(state["job"]["error"].as_str().unwrap().contains("context"));
}

#[test]
fn poisoned_state_prevents_startup_without_panicking() {
    let directory = tempfile::TempDir::new().unwrap();
    let shared = super::super::published::tests::poisoned();
    let (sender, _receiver) = mpsc::sync_channel(1);
    assert!(initialize(&options(directory.path()), &shared, &sender).is_none());
}

#[test]
fn publication_failure_stops_idle_refresh_instead_of_looping() {
    let completed = crate::test_support::run_with_timeout(Duration::from_secs(2), || {
        let directory = tempfile::TempDir::new().unwrap();
        let mut runtime =
            Runtime::new(directory.path(), Size::new(80, 24), Profile::Isolated).unwrap();
        let shared = super::super::published::tests::poisoned();
        let host = HeadlessHost::new();
        host.resize(90, 30).unwrap();
        let _scope = host.enter();
        let (_sender, receiver) = mpsc::sync_channel(1);
        drive(
            &mut runtime,
            &receiver,
            &shared,
            &host,
            &AtomicBool::new(false),
        );
        assert_eq!(runtime.terminal.backend().buffer().area.width, 90);
        assert!(shared.snapshot().is_err());
    });
    assert_eq!(completed, Some(()));
}

#[test]
fn completed_operation_survives_failed_status_publication() {
    let directory = tempfile::TempDir::new().unwrap();
    let mut runtime = Runtime::new(directory.path(), Size::new(80, 24), Profile::Isolated).unwrap();
    let shared = super::super::published::tests::poisoned();
    let host = HeadlessHost::new();
    let _scope = host.enter();
    execute(
        &mut runtime,
        &shared,
        &host,
        Command {
            id: super::super::published::JobId(1),
            operation: Operation::Resize(Size::new(88, 26)),
        },
    );
    assert_eq!(runtime.terminal.backend().buffer().area.width, 88);
    assert!(shared.snapshot().is_err());
}
