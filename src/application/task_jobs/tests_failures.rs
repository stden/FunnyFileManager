//! Purpose: Tests for what the controller does when sampling fails: a
//! failed sample shown and logged once, a worker that ends or cannot be
//! reached lost for the session and not started again, no host, a reader
//! that cannot open, and a thread that cannot start.

use std::io;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

use super::logged::Logged;
use super::tests::{Fixture, WAIT};
use super::Sampler;
use crate::application::Redraw;
use crate::models::{
    ControlError, Health, Overlay, PriorityChange, ProcessHost, ProcessKey, ProcessReader, Sample,
    SampleError, Schedule, Signal, SystemPart, Table, STALL_LIMIT, WARM_UP,
};
use crate::test_support::processes::one_core;

/// A host whose readers crash, ending their thread, and which counts them.
#[derive(Default)]
struct Crashing(AtomicUsize);

struct CrashingReader;

impl ProcessReader for CrashingReader {
    fn sample(&mut self) -> Result<Sample, SampleError> {
        panic!("this reader ends its thread");
    }
}

impl ProcessHost for Crashing {
    fn reader(&self) -> Result<Box<dyn ProcessReader>, SampleError> {
        self.0.fetch_add(1, Ordering::Relaxed);
        Ok(Box::new(CrashingReader))
    }

    fn signal(&self, _: ProcessKey, _: Signal) -> Result<(), ControlError> {
        Err(ControlError::Unsupported)
    }

    fn set_priority(&self, _: ProcessKey, _: PriorityChange) -> Result<(), ControlError> {
        Err(ControlError::Unsupported)
    }
}

/// A host that cannot open a reader for this system.
struct NoClock;

impl ProcessHost for NoClock {
    fn reader(&self) -> Result<Box<dyn ProcessReader>, SampleError> {
        Err(SampleError::Malformed(SystemPart::ClockRate))
    }

    fn signal(&self, _: ProcessKey, _: Signal) -> Result<(), ControlError> {
        Err(ControlError::Unsupported)
    }

    fn set_priority(&self, _: ProcessKey, _: PriorityChange) -> Result<(), ControlError> {
        Err(ControlError::Unsupported)
    }
}

#[test]
fn a_worker_that_ends_stops_the_session_and_is_not_started_again() {
    let host = Arc::new(Crashing::default());
    let installed: Arc<dyn ProcessHost> = Arc::<Crashing>::clone(&host);
    let mut fixture = Fixture::new(Some(installed));
    fixture.open();
    let t0 = Instant::now();
    fixture.pump(t0);
    fixture.pump_until_needed(t0);
    assert!(matches!(
        fixture.table(),
        Table::Failed(SampleError::WorkerStopped)
    ));
    assert_eq!(fixture.list().refresh().schedule(), Schedule::Stopped);
    assert_eq!(fixture.pump(t0 + STALL_LIMIT), Redraw::Unchanged);
    let opened = host.0.load(Ordering::Relaxed);
    assert_eq!(opened, 1, "no respawn until the list is reopened");
}

#[test]
fn without_a_host_the_table_says_so_once() {
    let mut fixture = Fixture::new(None);
    fixture.open();
    let t0 = Instant::now();
    assert_eq!(fixture.pump(t0), Redraw::Needed);
    assert!(matches!(
        fixture.table(),
        Table::Failed(SampleError::Unsupported)
    ));
    assert_eq!(fixture.pump(t0 + STALL_LIMIT), Redraw::Unchanged);
}

#[test]
fn a_reader_that_cannot_open_shows_why() {
    let mut fixture = Fixture::new(Some(Arc::new(NoClock)));
    fixture.open();
    assert_eq!(fixture.pump(Instant::now()), Redraw::Needed);
    let expected = SampleError::Malformed(SystemPart::ClockRate);
    assert!(matches!(fixture.table(), Table::Failed(error) if *error == expected));
}

#[test]
fn a_thread_that_cannot_start_leaves_the_reader_unavailable_for_the_session() {
    let (mut fixture, _host, _gate) = Fixture::gated();
    fixture.open();
    let Overlay::Tasks(list) = &mut fixture.app.overlay else {
        panic!("the task list is open");
    };
    let redraw = fixture
        .jobs
        .unavailable(list, &io::Error::other("no threads left"));
    assert_eq!(redraw, Redraw::Needed);
    assert!(matches!(
        fixture.table(),
        Table::Failed(SampleError::WorkerUnavailable)
    ));
    assert_eq!(fixture.list().refresh().schedule(), Schedule::Stopped);
}

/// A failing sample would otherwise write a line every refresh.
#[test]
fn each_error_is_logged_once_per_session() {
    let mut fixture = Fixture::new(None);
    let first = fixture.app.task_host.next_session();
    let second = fixture.app.task_host.next_session();
    let mut logged = Logged::default();
    let stopped = SampleError::WorkerStopped;
    let stalled = SampleError::WorkerStalled;
    assert!(logged.first(first, stopped));
    assert!(!logged.first(first, stopped));
    assert!(logged.first(first, stalled));
    assert!(logged.first(second, stopped));
}

#[test]
fn a_failed_sample_shows_its_error_and_is_logged_once() {
    let (mut fixture, _host, gate) = Fixture::gated();
    fixture.open();
    let t0 = Instant::now();
    fixture.pump(t0);
    let denied = SampleError::Unreadable {
        part: SystemPart::ProcessList,
        kind: io::ErrorKind::PermissionDenied,
    };
    gate.open(Err(denied));
    fixture.pump_until_needed(t0);
    assert!(matches!(fixture.table(), Table::Failed(error) if *error == denied));
    let session = fixture.list().session();
    assert!(
        !fixture.jobs.logged.first(session, denied),
        "already logged"
    );
}

/// A worker whose thread ended between this pass's drain and its request
/// is lost as the drain would have lost it.
#[test]
fn a_request_to_a_worker_that_has_gone_loses_it() {
    let host = Arc::new(Crashing::default());
    let installed: Arc<dyn ProcessHost> = Arc::<Crashing>::clone(&host);
    let mut fixture = Fixture::new(Some(installed));
    fixture.open();
    let t0 = Instant::now();
    fixture.pump(t0);
    let deadline = Instant::now() + WAIT;
    while !matches!(&fixture.jobs.sampler, Sampler::Running(worker) if worker.handle.is_finished())
    {
        assert!(
            Instant::now() < deadline,
            "the reader did not end its thread"
        );
        thread::yield_now();
    }
    let Fixture { app, jobs, .. } = &mut fixture;
    let Overlay::Tasks(list) = &mut app.overlay else {
        panic!("the task list is open");
    };
    list.apply_sample(Ok(one_core(Vec::new())), t0);
    assert_eq!(jobs.request(list, t0 + WARM_UP), Redraw::Needed);
    assert!(matches!(jobs.sampler, Sampler::Idle));
    assert_eq!(
        list.health(),
        Some(Health::Failed(SampleError::WorkerStopped))
    );
    assert_eq!(list.refresh().schedule(), Schedule::Stopped);
}

#[test]
fn abandoning_a_sampler_without_a_running_thread_changes_nothing() {
    let now = Instant::now();
    assert!(matches!(Sampler::Idle.abandon(now), Sampler::Idle));
}
