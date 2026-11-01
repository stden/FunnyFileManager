//! Purpose: Exercise probe-worker failure, session, and shutdown paths offline.
use super::*;
use crate::models::tasks::Ipv4;
use crate::models::Pid;
use crate::prompt::Privilege;
use crate::test_support::processes::one_core;

fn list(host: &mut TaskHostSlot, now: Instant) -> TaskList {
    let mut list = TaskList::new(
        host.next_session(),
        Pid::new(42).expect("pid"),
        Privilege::User,
    );
    let mut sample = one_core(Vec::new());
    sample.network.gateway = Some(Ipv4::new(127, 0, 0, 1));
    list.apply_sample(Ok(sample), now);
    list
}

fn fake_worker(
    session: TaskSession,
    flight: Flight,
) -> (ProbeWorker, SyncSender<Answers>, Receiver<Batch>) {
    let (requests, incoming) = mpsc::sync_channel(1);
    let (outgoing, results) = mpsc::sync_channel(1);
    let handle = thread::spawn(|| {});
    (
        ProbeWorker {
            session,
            requests,
            results,
            handle,
            flight,
        },
        outgoing,
        incoming,
    )
}

#[test]
fn missing_provider_disables_only_the_current_session() {
    let mut host = TaskHostSlot::default();
    let session = host.next_session();
    let mut runner = NetProbeRunner::default();
    assert!(!runner.ensure_worker(session, &host));
    assert!(matches!(runner.state, State::Disabled(current) if current == session));
    assert!(!runner.ensure_worker(session, &host));
    let next = host.next_session();
    assert!(!runner.ensure_worker(next, &host));
    assert!(matches!(runner.state, State::Disabled(current) if current == next));
}

#[test]
fn disconnected_results_disable_the_worker_for_its_session() {
    let now = Instant::now();
    let mut host = TaskHostSlot::default();
    let mut list = list(&mut host, now);
    let (worker, sender, _incoming) = fake_worker(list.session(), Flight::Since(now));
    drop(sender);
    let mut runner = NetProbeRunner {
        state: State::Running(worker),
    };
    assert_eq!(
        runner.drain(&mut list, &mut Performance::default(), now),
        Redraw::Unchanged
    );
    assert!(matches!(runner.state, State::Disabled(current) if current == list.session()));
}

#[test]
fn disconnected_request_disables_the_worker_after_a_due_batch() {
    let now = Instant::now();
    let mut host = TaskHostSlot::default();
    let mut list = list(&mut host, now);
    let (worker, _sender, incoming) = fake_worker(list.session(), Flight::Since(now));
    drop(incoming);
    let mut runner = NetProbeRunner {
        state: State::Running(worker),
    };
    assert!(!runner.ensure_worker(list.session(), &host));
    runner.send_due(&mut list, now);
    let State::Running(worker) = &mut runner.state else {
        panic!("worker");
    };
    worker.flight = Flight::Idle;
    runner.send_due(&mut list, now);
    assert!(matches!(runner.state, State::Disabled(current) if current == list.session()));
}

#[test]
fn full_request_queue_keeps_one_probe_in_flight() {
    let now = Instant::now();
    let mut host = TaskHostSlot::default();
    let mut list = list(&mut host, now);
    let (worker, _sender, incoming) = fake_worker(list.session(), Flight::Idle);
    worker.requests.try_send(Vec::new()).expect("fill queue");
    let mut runner = NetProbeRunner {
        state: State::Running(worker),
    };
    runner.send_due(&mut list, now);
    assert!(matches!(&runner.state, State::Running(worker) if worker.flight == Flight::Since(now)));
    assert_eq!(
        incoming.try_recv().expect("queued batch"),
        Vec::<ProbeTarget>::new()
    );
}

#[test]
fn new_session_retires_the_previous_worker() {
    let now = Instant::now();
    let mut host = TaskHostSlot::default();
    let old = host.next_session();
    let mut list = list(&mut host, now);
    let (worker, _sender, _incoming) = fake_worker(old, Flight::Idle);
    let mut runner = NetProbeRunner {
        state: State::Running(worker),
    };
    assert_eq!(
        runner.pump(&mut list, &host, &mut Performance::default(), now),
        Redraw::Unchanged
    );
    assert!(matches!(runner.state, State::Disabled(current) if current == list.session()));
}

#[test]
fn retiring_idle_and_busy_workers_keeps_ui_non_blocking() {
    let now = Instant::now();
    let mut host = TaskHostSlot::default();
    let session = host.next_session();
    let (worker, _sender, _incoming) = fake_worker(session, Flight::Idle);
    let mut runner = NetProbeRunner {
        state: State::Running(worker),
    };
    runner.retire();
    assert!(matches!(runner.state, State::Idle));
    runner.retire();
    assert!(matches!(runner.state, State::Idle));
    let (worker, _sender, _incoming) = fake_worker(session, Flight::Since(now));
    runner.state = State::Running(worker);
    runner.retire();
    assert!(matches!(runner.state, State::Abandoned(_)));
}

#[test]
fn an_abandoned_worker_prevents_a_second_worker_from_starting() {
    let mut host = TaskHostSlot::default();
    let session = host.next_session();
    let (release, wait) = mpsc::sync_channel::<()>(0);
    let handle = thread::spawn(move || wait.recv().expect("release"));
    let mut runner = NetProbeRunner {
        state: State::Abandoned(handle),
    };
    assert!(!runner.ensure_worker(session, &host));
    release.send(()).expect("release");
    let State::Abandoned(handle) = mem::take(&mut runner.state) else {
        panic!("abandoned worker");
    };
    handle.join().expect("join");
}

struct EmptyProber;
impl NetProber for EmptyProber {
    fn probe(&mut self, _targets: &[ProbeTarget]) -> Answers {
        Vec::new()
    }
}

#[test]
fn worker_stops_when_result_receiver_is_gone() {
    let (requests, incoming) = mpsc::sync_channel(1);
    let (outgoing, receiver) = mpsc::sync_channel(1);
    drop(receiver);
    requests.send(Vec::new()).expect("request");
    serve_probes(Box::new(EmptyProber), &incoming, &outgoing);
}
