//! Purpose: Process controls cannot accept samples taken before their effects;
//! fixture hosts record actions and synthetic instants drive refresh transitions.

use std::time::Instant;

use super::{confirm_terminate, press, step_nice};
use crate::models::{
    Arrival, Cadence, ConfirmButton, Health, KernelPriority, NiceStep, Priority, PriorityChange,
    ProcessSample, Request, SampleError, Schedule, REFRESH_INTERVAL,
};
use crate::prompt::Privilege;
use crate::test_support::processes::{one_core, Control};
use crate::test_support::task_list::{key_of, named, nice, TaskApp};

fn at_nice(value: i32) -> ProcessSample {
    ProcessSample {
        priority: Priority::Unix {
            kernel: KernelPriority::new(20),
            nice: nice(value),
        },
        ..named(4242, "firefox")
    }
}

fn pending(cadence: Cadence, request: Request) -> (TaskApp, Instant) {
    let mut tasks = TaskApp::open(vec![at_nice(0)], Privilege::Root);
    let now = Instant::now() + REFRESH_INTERVAL;
    if request == Request::Explicit {
        tasks.list().refresh_now();
    }
    assert_eq!(tasks.list().take_request(now), Some(request));
    if cadence == Cadence::Held {
        tasks.list().toggle_hold();
    }
    (tasks, now)
}

fn shown_nice(tasks: &mut TaskApp, expected: i32) {
    let priority = tasks.list().row(key_of(4242)).unwrap().sample().priority;
    assert!(matches!(priority, Priority::Unix { nice: value, .. } if value == nice(expected)));
}

#[test]
fn a_pre_control_sample_cannot_undo_accepted_nice_values() {
    for cadence in [Cadence::Live, Cadence::Held] {
        let (mut tasks, now) = pending(cadence, Request::Automatic);
        step_nice(&mut tasks.app, NiceStep::Lower);
        step_nice(&mut tasks.app, NiceStep::Lower);
        shown_nice(&mut tasks, 2);
        tasks
            .list()
            .apply_sample(Ok(one_core(vec![at_nice(0)])), now);
        shown_nice(&mut tasks, 2);
        assert_eq!(tasks.list().take_request(now), Some(Request::Explicit));
        tasks
            .list()
            .apply_sample(Ok(one_core(vec![at_nice(2)])), now);
        shown_nice(&mut tasks, 2);
        assert_eq!(tasks.list().refresh().cadence(), cadence);
        step_nice(&mut tasks.app, NiceStep::Lower);
        assert_eq!(
            tasks.host.controls().last(),
            Some(&Control::Priority(
                key_of(4242),
                PriorityChange::Nice(nice(3))
            ))
        );
    }
}

#[test]
fn a_pre_signal_sample_is_followed_by_an_explicit_read_while_held() {
    let (mut tasks, now) = pending(Cadence::Held, Request::Automatic);
    confirm_terminate(tasks.list());
    press(&mut tasks.app, ConfirmButton::Terminate);
    tasks
        .list()
        .apply_sample(Ok(one_core(vec![at_nice(0)])), now);
    assert_eq!(tasks.list().take_request(now), Some(Request::Explicit));
    tasks.list().apply_sample(Ok(one_core(Vec::new())), now);
    assert_eq!(tasks.list().rows().len(), 0);
    assert_eq!(tasks.list().refresh().schedule(), Schedule::Held);
}

#[test]
fn controls_queue_one_new_sample_without_starting_beside_the_old_one() {
    for cadence in [Cadence::Live, Cadence::Held] {
        for request in [Request::Automatic, Request::Explicit] {
            let (mut tasks, now) = pending(cadence, request);
            step_nice(&mut tasks.app, NiceStep::Lower);
            step_nice(&mut tasks.app, NiceStep::Lower);
            let mut refresh = tasks.list().refresh();
            refresh.refresh_now();
            refresh.toggle_hold();
            refresh.toggle_hold();
            assert_eq!(refresh.in_flight_since(), Some(now));
            assert_eq!(refresh.take_request(now), None);
            assert_eq!(refresh.completed(now, REFRESH_INTERVAL), Arrival::Discard);
            assert_eq!(refresh.take_request(now), Some(Request::Explicit));
            assert_eq!(refresh.take_request(now), None);
            assert_eq!(refresh.completed(now, REFRESH_INTERVAL), Arrival::Apply);
            assert_eq!(refresh.cadence(), cadence);
        }
    }
}

#[test]
fn a_stalled_pre_control_sample_keeps_the_followup_due_even_while_held() {
    for cadence in [Cadence::Live, Cadence::Held] {
        let (mut tasks, now) = pending(cadence, Request::Automatic);
        step_nice(&mut tasks.app, NiceStep::Lower);
        tasks.list().mark_stalled();
        assert_eq!(tasks.list().health(), Some(Health::Stalled));
        assert_eq!(tasks.list().take_request(now), Some(Request::Explicit));
        tasks
            .list()
            .apply_sample(Ok(one_core(vec![at_nice(1)])), now);
        assert_eq!(tasks.list().health(), Some(Health::Fresh));
    }
}

#[test]
fn a_control_cannot_restart_a_worker_that_stopped() {
    let (mut tasks, now) = pending(Cadence::Held, Request::Automatic);
    tasks.list().worker_lost(SampleError::WorkerStopped);
    step_nice(&mut tasks.app, NiceStep::Lower);
    tasks
        .list()
        .apply_sample(Ok(one_core(vec![at_nice(0)])), now);
    shown_nice(&mut tasks, 1);
    assert_eq!(tasks.list().refresh().schedule(), Schedule::Stopped);
    assert_eq!(tasks.list().take_request(now), None);
}
