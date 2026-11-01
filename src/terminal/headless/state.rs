//! Purpose: Typed process state and bounded private terminal output.

use std::{collections::VecDeque, sync::mpsc::SyncSender};

pub(super) const OUTPUT_LIMIT: usize = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalState {
    Idle,
    Running,
    Exited { code: u32 },
    Cancelled,
    Failed,
}

#[derive(Debug, Clone)]
pub struct HeadlessSnapshot {
    pub state: ExternalState,
    pub output: String,
    pub dropped_bytes: u64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Generation(pub u64);

pub(super) enum SessionState {
    Ready(ExternalState),
    Running(SyncSender<Vec<u8>>),
}

impl SessionState {
    pub fn public(&self) -> ExternalState {
        match self {
            Self::Ready(state) => *state,
            Self::Running(_) => ExternalState::Running,
        }
    }
}

#[derive(Default)]
pub(super) struct Captured {
    bytes: VecDeque<u8>,
    pub dropped: u64,
}

impl Captured {
    pub fn append(&mut self, bytes: &[u8]) {
        let excess = self
            .bytes
            .len()
            .saturating_add(bytes.len())
            .saturating_sub(OUTPUT_LIMIT);
        self.dropped = self
            .dropped
            .saturating_add(u64::try_from(excess).unwrap_or(u64::MAX));
        let remove = excess.min(self.bytes.len());
        self.bytes.drain(..remove);
        self.bytes
            .extend(bytes.iter().skip(excess.saturating_sub(remove)).copied());
    }

    pub fn bytes(&self) -> Vec<u8> {
        self.bytes.iter().copied().collect()
    }
}

pub(super) struct Session {
    pub state: SessionState,
    pub output: Captured,
    pub generation: Generation,
    pub size: super::size::PrivateSize,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            state: SessionState::Ready(ExternalState::Idle),
            output: Captured::default(),
            generation: Generation(0),
            size: super::size::PrivateSize::default(),
        }
    }
}
