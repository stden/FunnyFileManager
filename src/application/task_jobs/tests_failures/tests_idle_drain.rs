//! Purpose: Verify idle-flight sample drains and duplicate error log suppression.

use std::sync::mpsc;
use std::thread;
use std::time::Instant;

use super::super::tests::{answer, Fixture};
use super::super::worker::{Flight, Worker};
use super::super::Sampler;
use crate::application::Redraw;
use crate::models::{Overlay, SampleError};

#[test]
fn draining_an_idle_flight_worker_applies_sample_without_timing_record() {
    let mut fixture = Fixture::new(None);
    fixture.open();
    let (requests, _incoming) = mpsc::sync_channel(1);
    let (outgoing, results) = mpsc::sync_channel(1);
    outgoing.send(answer()).expect("send sample");
    let session = fixture.list().session();
    fixture.jobs.sampler = Sampler::Running(Worker {
        session,
        requests,
        results,
        handle: thread::spawn(|| {}),
        flight: Flight::Idle,
    });
    let Overlay::Tasks(list) = &mut fixture.app.overlay else {
        panic!("the task list is open");
    };
    assert_eq!(
        fixture
            .jobs
            .drain(list, &mut fixture.app.performance, Instant::now()),
        Redraw::Needed
    );
    assert!(list.snapshot().is_some());
}

#[test]
fn log_once_suppresses_repeated_errors_in_the_same_session() {
    let mut fixture = Fixture::new(None);
    fixture.open();
    let session = fixture.list().session();
    fixture.jobs.log_once(session, SampleError::WorkerStopped);
    fixture.jobs.log_once(session, SampleError::WorkerStopped);
    assert!(!fixture
        .jobs
        .logged
        .first(session, SampleError::WorkerStopped));
}
