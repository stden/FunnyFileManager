//! Purpose: The gated reader of the fake process table: each sample blocks
//! until the test lets one through its gate, which is how a test holds a
//! sample in flight, late or stuck without sleeping.

use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::time::Duration;

use super::lock;
use crate::models::{ProcessReader, Sample, SampleError};

/// A reader that blocks in every sample until its [`Gate`] lets one
/// through.
pub struct GatedReader(Arc<GateState>);

impl GatedReader {
    pub(super) const fn new(state: Arc<GateState>) -> Self {
        Self(state)
    }
}

impl ProcessReader for GatedReader {
    /// The next answer the gate lets through; the stopped worker's error
    /// once the gate is gone.
    #[expect(
        clippy::significant_drop_tightening,
        reason = "the guard moves into the condition variable's wait, which the lint does not follow"
    )]
    fn sample(&mut self) -> Result<Sample, SampleError> {
        let state = &self.0;
        let mut gated = lock(&state.gated);
        gated.entered = gated.entered.saturating_add(1);
        state.changed.notify_all();
        let mut gated = state
            .changed
            .wait_while(gated, |gated| {
                gated.answers.is_empty() && gated.test_side == TestSide::Held
            })
            .unwrap_or_else(PoisonError::into_inner);
        let answer = gated.answers.pop_front();
        drop(gated);
        answer.unwrap_or(Err(SampleError::WorkerStopped))
    }
}

/// The test's side of the gated readers. Dropping it releases every reader
/// still waiting, with an error.
pub struct Gate(Arc<GateState>);

/// What a gate and its readers share.
#[derive(Default)]
pub(super) struct GateState {
    gated: Mutex<Gated>,
    changed: Condvar,
}

#[derive(Default)]
struct Gated {
    /// Samples begun, answered or not.
    entered: usize,
    answers: VecDeque<Result<Sample, SampleError>>,
    test_side: TestSide,
}

/// Whether the test still holds its [`Gate`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum TestSide {
    #[default]
    Held,
    Dropped,
}

impl Gate {
    pub(super) const fn new(state: Arc<GateState>) -> Self {
        Self(state)
    }

    /// Lets one sample through with `answer`: the one waiting, or the next.
    pub fn open(&self, answer: Result<Sample, SampleError>) {
        lock(&self.0.gated).answers.push_back(answer);
        self.0.changed.notify_all();
    }

    /// How many samples the readers have begun.
    #[must_use]
    pub fn entered(&self) -> usize {
        lock(&self.0.gated).entered
    }

    /// Waits until the readers have begun `count` samples; `false` when
    /// `timeout` passes first.
    #[must_use]
    #[expect(
        clippy::significant_drop_tightening,
        reason = "the guard moves into the condition variable's wait, which the lint does not follow"
    )]
    pub fn wait_for_entries(&self, count: usize, timeout: Duration) -> bool {
        let gated = lock(&self.0.gated);
        let (gated, _) = self
            .0
            .changed
            .wait_timeout_while(gated, timeout, |gated| gated.entered < count)
            .unwrap_or_else(PoisonError::into_inner);
        let reached = gated.entered >= count;
        drop(gated);
        reached
    }
}

impl Drop for Gate {
    fn drop(&mut self) {
        lock(&self.0.gated).test_side = TestSide::Dropped;
        self.0.changed.notify_all();
    }
}
