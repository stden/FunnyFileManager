//! Purpose: A worker that cannot start, and one that stops without an answer,
//! free the request queue and tell the user, so the next request can run.

use std::sync::mpsc;
use std::thread::Builder;

use super::{app_with_database, press_right};
use crate::application::database_jobs::{DatabaseJobs, Running};
use crate::application::Redraw;
use crate::status::Status;

#[test]
fn a_worker_that_cannot_start_frees_the_queue_and_says_so() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let mut app = app_with_database(&temp);
    let mut jobs = DatabaseJobs::default();
    press_right(&mut app);
    let request = app.database_requests.take().expect("a queued request");

    // No system can reserve a stack this big, so the thread is never started.
    let worker = Builder::new().stack_size(1 << (usize::BITS - 2));
    jobs.start(&mut app, request, worker);

    assert!(jobs.running.is_none());
    assert!(app.database_requests.is_idle());
    assert_eq!(
        app.status,
        Status::Error("Cannot start database worker".to_owned())
    );
}

#[test]
fn a_worker_that_stops_without_an_answer_frees_the_queue_and_says_so() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let mut app = app_with_database(&temp);
    let mut jobs = DatabaseJobs::default();
    press_right(&mut app);
    let request = app.database_requests.take().expect("a queued request");
    let (sender, receiver) = mpsc::channel();
    jobs.running = Some(Running {
        id: request.id,
        receiver,
    });
    drop(sender); // The worker is gone and sent nothing.

    assert_eq!(jobs.pump(&mut app), Redraw::Needed);

    assert!(jobs.running.is_none());
    assert!(app.database_requests.is_idle());
    assert_eq!(
        app.status,
        Status::Error("Database worker stopped unexpectedly".to_owned())
    );
}

#[test]
fn a_worker_still_at_work_leaves_the_screen_and_the_queue_alone() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let mut app = app_with_database(&temp);
    let mut jobs = DatabaseJobs::default();
    press_right(&mut app);
    let request = app.database_requests.take().expect("a queued request");
    let (sender, receiver) = mpsc::channel();
    jobs.running = Some(Running {
        id: request.id,
        receiver,
    });

    assert_eq!(jobs.pump(&mut app), Redraw::Unchanged);

    assert!(jobs.running.is_some());
    assert!(!app.database_requests.is_idle());
    drop(sender);
}
