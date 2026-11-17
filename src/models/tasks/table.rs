//! Purpose: The process table as far as a task list has one: none yet, a
//! snapshot and how fresh it is, or the error that came before any.

use super::{Health, ProcessRow, SampleError, Snapshot};

/// The process table as far as the list has one.
#[derive(Clone)]
pub enum Table {
    /// No sample has arrived yet this session.
    Waiting,
    /// Boxed: a snapshot is large, and the other states are a word.
    Ready {
        snapshot: Box<Snapshot>,
        health: Health,
    },
    /// Sampling failed before any snapshot existed.
    Failed(SampleError),
}

impl Table {
    /// Every row of the snapshot; empty without one.
    #[must_use]
    pub fn rows(&self) -> &[ProcessRow] {
        match self {
            Self::Ready { snapshot, .. } => snapshot.rows(),
            Self::Waiting | Self::Failed(_) => &[],
        }
    }
}
