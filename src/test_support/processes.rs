//! Purpose: A fake process table for the tests above the ports — the task
//! list's controller, ops, popup, rendering and BDD scenarios — and the
//! builders of the samples it serves.
//!
//! [`FixtureHost`] hands out readers that return canned samples in turn and
//! records every signal and renice instead of sending it, so no test that
//! uses it can reach a real process. [`FixtureHost::gated`] hands out
//! readers that block until the test lets a sample through its [`Gate`],
//! which is how a test holds a sample in flight, late or stuck. Fixture
//! processes belong to `bob` and run from `/usr/bin`, which the private-data
//! lint accepts.

mod gate;
mod samples;

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use gate::GateState;
pub use gate::{Gate, GatedReader};
pub use samples::{counters, one_core, process, sample, system};

use crate::models::{
    ControlError, PriorityChange, ProcessHost, ProcessKey, ProcessReader, Sample, SampleError,
    Signal,
};

/// What a fixture host was asked to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    Signal(ProcessKey, Signal),
    Priority(ProcessKey, PriorityChange),
}

/// A process host that samples nothing real and signals nothing at all.
///
/// Clones share one fixture: a test installs one clone on `App` and asks
/// another what was recorded.
#[derive(Clone)]
pub struct FixtureHost {
    fixture: Arc<Mutex<Fixture>>,
    readers: Readers,
}

/// What the host's readers serve.
#[derive(Clone)]
enum Readers {
    /// The canned samples, in turn.
    Canned,
    /// Whatever the test lets through the gate.
    Gated(Arc<GateState>),
}

#[derive(Default)]
struct Fixture {
    samples: Vec<Sample>,
    /// How many canned samples have been served.
    served: usize,
    controls: Vec<Control>,
    /// Processes a signal ended: gone from every later canned sample.
    ended: BTreeSet<ProcessKey>,
    /// The answer a signal or renice of a process gets instead of `Ok`.
    failures: Vec<(ProcessKey, ControlError)>,
    readers_opened: usize,
}

impl FixtureHost {
    /// A host whose readers return `samples` in turn, the last one again
    /// and again; with none, every sample fails as unsupported.
    #[must_use]
    pub fn new(samples: Vec<Sample>) -> Self {
        let fixture = Fixture {
            samples,
            ..Fixture::default()
        };
        Self {
            fixture: Arc::new(Mutex::new(fixture)),
            readers: Readers::Canned,
        }
    }

    /// A host whose readers block on the returned gate: each sample waits
    /// until the test opens it with an answer.
    #[must_use]
    pub fn gated() -> (Self, Gate) {
        let gate = Arc::new(GateState::default());
        let host = Self {
            fixture: Arc::default(),
            readers: Readers::Gated(Arc::clone(&gate)),
        };
        (host, Gate::new(gate))
    }

    /// Every signal or renice of `key` from now on answers `error`; it is
    /// still recorded.
    pub fn fail(&self, key: ProcessKey, error: ControlError) {
        self.fixture().failures.push((key, error));
    }

    /// What the host was asked to do, in order.
    #[must_use]
    pub fn controls(&self) -> Vec<Control> {
        self.fixture().controls.clone()
    }

    /// How many readers the host has handed out: one per sampling worker.
    #[must_use]
    pub fn readers_opened(&self) -> usize {
        self.fixture().readers_opened
    }

    fn fixture(&self) -> MutexGuard<'_, Fixture> {
        lock(&self.fixture)
    }

    /// Records `control` and gives the answer scripted for `key`.
    fn record(&self, key: ProcessKey, control: Control) -> Result<(), ControlError> {
        let mut fixture = self.fixture();
        fixture.controls.push(control);
        let failure = fixture.failures.iter().find(|(failed, _)| *failed == key);
        let answer = failure.map_or(Ok(()), |(_, error)| Err(*error));
        drop(fixture);
        answer
    }
}

impl ProcessHost for FixtureHost {
    fn reader(&self) -> Result<Box<dyn ProcessReader>, SampleError> {
        let mut fixture = self.fixture();
        fixture.readers_opened = fixture.readers_opened.saturating_add(1);
        drop(fixture);
        Ok(match &self.readers {
            Readers::Canned => Box::new(FixtureReader(Arc::clone(&self.fixture))),
            Readers::Gated(gate) => Box::new(GatedReader::new(Arc::clone(gate))),
        })
    }

    /// Recorded, never sent. SIGTERM and SIGKILL end the fixture process:
    /// later canned samples leave it out.
    fn signal(&self, key: ProcessKey, signal: Signal) -> Result<(), ControlError> {
        self.record(key, Control::Signal(key, signal))?;
        if matches!(signal, Signal::Terminate | Signal::Kill) {
            self.fixture().ended.insert(key);
        }
        Ok(())
    }

    /// Recorded, never applied.
    fn set_priority(&self, key: ProcessKey, change: PriorityChange) -> Result<(), ControlError> {
        self.record(key, Control::Priority(key, change))
    }
}

/// A reader of a [`FixtureHost`]'s canned samples.
pub struct FixtureReader(Arc<Mutex<Fixture>>);

impl ProcessReader for FixtureReader {
    fn sample(&mut self) -> Result<Sample, SampleError> {
        let mut fixture = lock(&self.0);
        let at = fixture.served;
        fixture.served = at.saturating_add(1);
        let canned = fixture.samples.get(at).or_else(|| fixture.samples.last());
        let mut sample = canned.cloned().ok_or(SampleError::Unsupported)?;
        let ended = fixture.ended.clone();
        drop(fixture);
        sample
            .processes
            .retain(|process| !ended.contains(&process.key));
        Ok(sample)
    }
}

/// The fixture state, even after a test thread panicked holding it.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests;
