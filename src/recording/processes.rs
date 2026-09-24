//! Purpose: The process host a session gets: the real one live, and one that
//! reads but never signals or renices during a replay.
//!
//! A recording holds keys, not the processes they were aimed at. Replayed on
//! another machine, or later on this one, F8 Enter would signal whatever
//! process happens to sit under the cursor there (spec S8), so a replay sees
//! the processes and changes none of them.

use std::sync::Arc;

use super::Input;
use crate::models::{
    ControlError, PriorityChange, ProcessHost, ProcessKey, ProcessReader, SampleError, Signal,
};

impl Input {
    /// `inner` for a live session, recorded or not; read-only for a replay.
    pub(crate) fn process_host(&self, inner: Arc<dyn ProcessHost>) -> Arc<dyn ProcessHost> {
        match self {
            Self::Live(_) => inner,
            Self::Replay(_) => Arc::new(ReadOnlyHost(inner)),
        }
    }
}

/// A host that samples through the one it wraps and refuses every control.
struct ReadOnlyHost(Arc<dyn ProcessHost>);

impl ProcessHost for ReadOnlyHost {
    fn reader(&self) -> Result<Box<dyn ProcessReader>, SampleError> {
        self.0.reader()
    }

    fn signal(&self, _key: ProcessKey, _signal: Signal) -> Result<(), ControlError> {
        Err(ControlError::ReadOnly)
    }

    fn set_priority(&self, _key: ProcessKey, _change: PriorityChange) -> Result<(), ControlError> {
        Err(ControlError::ReadOnly)
    }
}

#[cfg(test)]
mod tests;
