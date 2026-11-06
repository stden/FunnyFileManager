//! Purpose: Net-probe worker spawn failure, per-session restart, and inactive `send_due` guards.

use super::*;
use crate::models::{
    ControlError, PriorityChange, ProcessHost, ProcessKey, ProcessReader, SampleError,
};
use std::sync::Arc;

struct StubHost;

impl ProcessHost for StubHost {
    fn reader(&self) -> Result<Box<dyn ProcessReader>, SampleError> {
        Err(SampleError::Unsupported)
    }

    fn signal(&self, _: ProcessKey, _: crate::models::Signal) -> Result<(), ControlError> {
        Ok(())
    }

    fn set_priority(&self, _: ProcessKey, _: PriorityChange) -> Result<(), ControlError> {
        Ok(())
    }

    fn net_prober(&self) -> Option<Box<dyn NetProber>> {
        Some(Box::new(EmptyProber))
    }
}

#[test]
fn spawn_failure_disables_session_and_idle_send_due_is_noop() {
    let now = Instant::now();
    let mut host = TaskHostSlot::default();
    host.install(Arc::new(StubHost));
    let mut task_list = list(&mut host, now);
    let session = task_list.session();

    let mut runner = NetProbeRunner::default();
    runner.send_due(&mut task_list, now);
    assert!(matches!(runner.state, State::Idle));

    FORCE_SPAWN_FAILURE.set(true);
    let started = runner.ensure_worker(session, &host);
    FORCE_SPAWN_FAILURE.set(false);
    assert!(!started);
    assert!(matches!(runner.state, State::Disabled(disabled) if disabled == session));
}

#[test]
fn a_failed_start_disables_only_the_current_task_session() {
    let mut host = TaskHostSlot::default();
    host.install(Arc::new(StubHost));
    let session = host.next_session();
    let mut runner = NetProbeRunner::default();
    FORCE_SPAWN_FAILURE.set(true);
    let started = runner.ensure_worker(session, &host);
    FORCE_SPAWN_FAILURE.set(false);
    assert!(!started);
    assert!(!runner.ensure_worker(session, &host));
    assert!(matches!(runner.state, State::Disabled(current) if current == session));
    let next = host.next_session();
    assert!(runner.ensure_worker(next, &host));
    assert!(matches!(&runner.state, State::Running(worker) if worker.session == next));
}

#[test]
fn disabled_probe_workers_ignore_due_requests() {
    let now = Instant::now();
    let mut host = TaskHostSlot::default();
    let mut task_list = list(&mut host, now);
    let mut runner = NetProbeRunner {
        state: State::Disabled(task_list.session()),
    };
    runner.send_due(&mut task_list, now);
    assert!(matches!(runner.state, State::Disabled(current) if current == task_list.session()));
    assert!(task_list.take_probe_request(now).is_some());
}
