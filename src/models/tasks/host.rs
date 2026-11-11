//! Purpose: The slot the process host lives in on `App` — installed once by
//! the production entry point, absent in tests that do not install one — and
//! the counter that gives every opening of the task list its own session.

use std::sync::Arc;

use super::{ProcessHost, TaskSession};

/// The platform's process table, if this build has one, and the sessions
/// minted so far.
///
/// `Clone` keeps the host: it is shared and never changes after installing,
/// so two clones of `App` control processes through the same one.
#[derive(Clone, Default)]
pub struct TaskHostSlot {
    host: Option<Arc<dyn ProcessHost>>,
    /// Sessions minted so far; session 0 is the default list's and is never
    /// minted.
    sessions: u64,
}

impl TaskHostSlot {
    /// Install the host every later opening reads and controls through.
    pub fn install(&mut self, host: Arc<dyn ProcessHost>) {
        self.host = Some(host);
    }

    /// The installed host, or `None` on a platform without one — then the
    /// task list falls back to the text report.
    #[must_use]
    pub fn host(&self) -> Option<Arc<dyn ProcessHost>> {
        self.host.clone()
    }

    /// A session number no earlier opening of this slot has had.
    pub fn next_session(&mut self) -> TaskSession {
        self.sessions = self.sessions.saturating_add(1);
        TaskSession::new(self.sessions)
    }
}
