//! Purpose: Which sample errors the task list's controller has logged,
//! so a table that fails on every refresh writes one line per session and
//! kind of error, not one per refresh.

use crate::models::{SampleError, TaskSession};

/// The sample errors logged so far, for the session that met them.
#[derive(Default)]
pub(super) struct Logged {
    session: Option<TaskSession>,
    errors: Vec<SampleError>,
}

impl Logged {
    /// Whether this is the first time `session` meets `error`; remembered
    /// either way.
    pub(super) fn first(&mut self, session: TaskSession, error: SampleError) -> bool {
        if self.session != Some(session) {
            self.session = Some(session);
            self.errors.clear();
        }
        if self.errors.contains(&error) {
            return false;
        }
        self.errors.push(error);
        true
    }
}
