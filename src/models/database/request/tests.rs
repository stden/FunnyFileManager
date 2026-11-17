//! Purpose: Pin the one-slot request queue: a request queues only while the
//! queue is idle, a worker takes a queued request and only that, and only the
//! running request can be finished.

use super::{Request, Requests, Task};
use crate::models::database::{ConnectionSpec, Driver, Name};

fn schemas_task() -> Task {
    Task::Schemas(ConnectionSpec {
        name: Name::new("local").expect("a name"),
        driver: Driver::Sqlite {
            path: "/data/x.db".into(),
        },
    })
}

/// The id of a request taken from a queue of its own, a stranger to the others.
fn stranger_id() -> super::RequestId {
    let mut other = Requests::default();
    let id = other.queue(schemas_task()).expect("a first id");
    assert!(other.take().is_some());
    assert!(other.finish(id));
    other.queue(schemas_task()).expect("a second id")
}

#[test]
fn a_request_queues_only_while_the_queue_is_idle() {
    let mut requests = Requests::default();
    assert!(requests.is_idle());
    let first = requests
        .queue(schemas_task())
        .expect("an idle queue accepts");
    assert!(!requests.is_idle());
    assert_eq!(requests.queue(schemas_task()), None, "a request is queued");

    requests.take().expect("the queued request");
    assert_eq!(requests.queue(schemas_task()), None, "a request is running");

    assert!(requests.finish(first));
    assert!(requests.is_idle());
    let second = requests.queue(schemas_task()).expect("idle again");
    assert_ne!(second, first, "every request has an id of its own");
}

#[test]
fn a_counter_with_no_ids_left_queues_nothing() {
    let mut requests = Requests {
        next: u64::MAX,
        ..Requests::default()
    };
    assert_eq!(requests.queue(schemas_task()), None);
    assert!(requests.is_idle());
}

#[test]
fn a_worker_takes_the_queued_request_once() {
    let mut requests = Requests::default();
    assert!(requests.take().is_none(), "nothing is queued yet");
    assert!(requests.is_idle());

    let id = requests.queue(schemas_task()).expect("queued");
    let Request { id: taken, task } = requests.take().expect("the queued request");
    assert_eq!(taken, id);
    assert!(matches!(task, Task::Schemas(_)));

    assert!(
        requests.take().is_none(),
        "a running request is not taken again"
    );
    assert!(!requests.is_idle());
    assert!(requests.finish(id), "taking it again left it running");
}

#[test]
fn only_the_running_request_can_be_finished() {
    let mut requests = Requests::default();
    let id = requests.queue(schemas_task()).expect("queued");
    assert!(!requests.finish(id), "a queued request is not running yet");

    requests.take().expect("the queued request");
    assert!(!requests.finish(stranger_id()), "another queue's id");
    assert!(!requests.is_idle());

    assert!(requests.finish(id));
    assert!(!requests.finish(id), "a request finishes once");
}
