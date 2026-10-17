//! Purpose: Self-tests of the fake process table: canned samples advance and
//! the last one repeats, controls are recorded and never sent, a terminated
//! process leaves later samples, scripted failures answer for their process
//! only, and a gated reader blocks until the test lets a sample through.

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use super::{one_core, process, Control, FixtureHost};
use crate::models::{
    ControlError, Niceness, Pid, PriorityChange, ProcessHost, ProcessKey, Sample, SampleError,
    Signal,
};

const WAIT: Duration = Duration::from_secs(5);

fn pid(raw: u32) -> Pid {
    Pid::new(raw).expect("a fixture PID")
}

fn key(raw: u32) -> ProcessKey {
    process(pid(raw), "bob").key
}

fn names(sample: &Sample) -> Vec<&str> {
    sample
        .processes
        .iter()
        .map(|row| row.name.as_str())
        .collect()
}

fn take(host: &FixtureHost) -> Result<Sample, SampleError> {
    host.reader().expect("a fixture reader").sample()
}

#[test]
fn canned_samples_advance_and_the_last_one_repeats() {
    let host = FixtureHost::new(vec![
        one_core(vec![process(pid(1), "init")]),
        one_core(vec![process(pid(2), "bash")]),
    ]);
    let mut reader = host.reader().expect("a fixture reader");
    let mut next = || names(&reader.sample().expect("a sample")).join(",");
    assert_eq!([next(), next(), next()], ["init", "bash", "bash"]);
    assert_eq!(host.readers_opened(), 1);
}

#[test]
fn a_host_without_samples_fails_every_sample() {
    let host = FixtureHost::new(Vec::new());
    assert_eq!(take(&host).err(), Some(SampleError::Unsupported));
}

#[test]
fn every_control_is_recorded_in_order_and_answered() {
    let host = FixtureHost::new(Vec::new());
    let installed: &dyn ProcessHost = &host.clone();
    let nice = PriorityChange::Nice(Niceness::MAX);
    assert_eq!(installed.signal(key(42), Signal::Stop), Ok(()));
    assert_eq!(installed.set_priority(key(43), nice), Ok(()));
    assert_eq!(
        host.controls(),
        [
            Control::Signal(key(42), Signal::Stop),
            Control::Priority(key(43), nice),
        ]
    );
}

/// SIGTERM and SIGKILL end the fixture process; SIGSTOP only pauses it.
#[test]
fn a_terminated_or_killed_process_is_gone_from_later_samples() {
    let processes = vec![
        process(pid(42), "firefox"),
        process(pid(43), "bash"),
        process(pid(44), "vim"),
    ];
    let host = FixtureHost::new(vec![one_core(processes)]);
    for (raw, signal) in [
        (42, Signal::Terminate),
        (43, Signal::Kill),
        (44, Signal::Stop),
    ] {
        host.signal(key(raw), signal).expect("recorded");
    }
    assert_eq!(names(&take(&host).expect("a sample")), ["vim"]);
}

#[test]
fn a_scripted_failure_answers_for_its_process_only() {
    let host = FixtureHost::new(vec![one_core(vec![process(pid(42), "firefox")])]);
    host.fail(key(42), ControlError::PermissionDenied);
    let refused = host.signal(key(42), Signal::Terminate);
    assert_eq!(refused, Err(ControlError::PermissionDenied));
    assert_eq!(host.signal(key(43), Signal::Terminate), Ok(()));
    assert_eq!(host.controls().len(), 2, "a failed control is still asked");
    assert_eq!(names(&take(&host).expect("a sample")), ["firefox"]);
}

#[test]
fn a_gated_reader_blocks_until_the_gate_lets_a_sample_through() {
    let (host, gate) = FixtureHost::gated();
    let mut reader = host.reader().expect("a gated reader");
    let (sender, results) = mpsc::channel();
    thread::spawn(move || sender.send(reader.sample()).expect("the test waits"));
    assert!(gate.wait_for_entries(1, WAIT), "the reader began a sample");
    assert_eq!(results.try_recv().err(), Some(mpsc::TryRecvError::Empty));
    gate.open(Ok(one_core(vec![process(pid(42), "firefox")])));
    let sample = results.recv_timeout(WAIT).expect("released");
    assert_eq!(names(&sample.expect("the sample let through")), ["firefox"]);
    assert_eq!(gate.entered(), 1);
}

#[test]
fn an_answer_let_through_early_is_served_to_the_next_sample() {
    let (host, gate) = FixtureHost::gated();
    gate.open(Err(SampleError::WorkerStalled));
    assert_eq!(take(&host).err(), Some(SampleError::WorkerStalled));
    assert!(!gate.wait_for_entries(2, Duration::ZERO));
}

#[test]
fn dropping_the_gate_releases_a_waiting_reader_with_an_error() {
    let (host, gate) = FixtureHost::gated();
    let mut reader = host.reader().expect("a gated reader");
    let (sender, results) = mpsc::channel();
    thread::spawn(move || sender.send(reader.sample()).expect("the test waits"));
    assert!(gate.wait_for_entries(1, WAIT));
    drop(gate);
    let answer = results.recv_timeout(WAIT).expect("released");
    assert_eq!(answer.err(), Some(SampleError::WorkerStopped));
}
