//! Purpose: Net-probe worker spawn failure and idle `send_due` guard.

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
