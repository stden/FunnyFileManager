//! Purpose: Identity-checked control against the real kernel. Every signal
//! and renice is aimed at a `sleep` child the test spawned and still holds:
//! the right process, a wrong start, a reaped child, the path without a
//! pidfd and an unknown start. The refusals, which share these helpers, are
//! in `tests_control_refusals.rs`.

use std::fs;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::time::Duration;

use super::control::{send, Pin, ProcessControl};
use super::parse::{self, ClockTicks};
use crate::models::{
    ControlError, NiceStep, Niceness, Pid, Priority, PriorityChange, ProcessKey, SettableClass,
    Signal, Start, StartTime,
};
use crate::test_support::sleeper::Sleeper;

/// Long enough for a signal that ends a process to have ended the child.
pub(super) const GRACE: Duration = Duration::from_millis(200);

pub(super) fn hertz() -> ClockTicks {
    ClockTicks::new(100).expect("USER_HZ is not zero")
}

pub(super) fn pid(raw: u32) -> Pid {
    Pid::new(raw).expect("a PID")
}

/// Control of the real `/proc`, run from `own`.
pub(super) fn control_by(own: Pid) -> ProcessControl {
    ProcessControl::new(PathBuf::from("/proc"), own, hertz())
}

/// Control of the real `/proc`, run from this test process.
pub(super) fn control() -> ProcessControl {
    control_by(pid(std::process::id()))
}

pub(super) fn stat_line(child: &Sleeper) -> String {
    fs::read_to_string(format!("/proc/{}/stat", child.pid())).expect("the child's stat")
}

fn stat_of(child: &Sleeper) -> parse::Stat {
    parse::stat(&stat_line(child), child.pid(), hertz()).expect("a stat line")
}

/// The child's start, in clock ticks since boot: `stat` field 22, read
/// apart from the parser under test.
fn start_ticks(child: &Sleeper) -> u64 {
    let line = stat_line(child);
    let (_, tail) = line.rsplit_once(") ").expect("a stat line");
    let field = tail.split(' ').nth(22 - 3).expect("a start field");
    field.parse().expect("a numeric start")
}

/// The child's identity, with the start time the kernel reports.
pub(super) fn key_of(child: &Sleeper) -> ProcessKey {
    ProcessKey::new(
        child.pid(),
        Start::Known(StartTime::new(start_ticks(child))),
    )
}

/// The child's PID with a start one tick later: another process.
fn successor_of(child: &Sleeper) -> ProcessKey {
    let later = StartTime::new(start_ticks(child) + 1);
    ProcessKey::new(child.pid(), Start::Known(later))
}

fn nice_of(child: &Sleeper) -> Niceness {
    match stat_of(child).priority {
        Priority::Unix { nice, .. } => nice,
        other => panic!("a Unix priority, not {other:?}"),
    }
}

#[test]
fn a_signal_to_the_same_process_is_delivered() {
    let mut child = Sleeper::spawn();
    assert_eq!(control().signal(key_of(&child), Signal::Terminate), Ok(()));
    assert_eq!(child.wait().signal(), Some(libc::SIGTERM));
}

#[test]
fn a_wrong_start_is_another_process_and_nothing_is_sent() {
    let mut child = Sleeper::spawn();
    let result = control().signal(successor_of(&child), Signal::Kill);
    assert_eq!(result, Err(ControlError::Replaced));
    assert!(child.outlives(GRACE));
}

/// Once the child is reaped its PID is free: the kernel answers that nothing
/// is there, or, should a newcomer already hold the PID, its start differs.
/// Either way nothing reaches the newcomer.
#[test]
fn a_reaped_process_is_gone_or_replaced() {
    let child = Sleeper::spawn();
    let key = key_of(&child);
    drop(child);
    let result = control().signal(key, Signal::Terminate);
    assert!(
        matches!(result, Err(ControlError::Gone | ControlError::Replaced)),
        "{result:?}"
    );
}

/// Without a pidfd the same check is followed by `kill(2)` on the PID.
#[test]
fn without_a_pidfd_the_signal_goes_by_pid() {
    let mut child = Sleeper::spawn();
    send(Pin::Unpinned, child.pid(), Signal::Terminate).expect("sent");
    assert_eq!(child.wait().signal(), Some(libc::SIGTERM));
}

#[test]
fn an_unknown_start_cannot_be_confirmed_and_nothing_is_sent() {
    let mut child = Sleeper::spawn();
    let unknown = ProcessKey::new(child.pid(), Start::Unknown);
    let result = control().signal(unknown, Signal::Kill);
    assert_eq!(result, Err(ControlError::Unverifiable));
    assert!(child.outlives(GRACE));
    let renice = control().set_priority(unknown, PriorityChange::Nice(Niceness::MAX));
    assert_eq!(renice, Err(ControlError::Unverifiable));
}

/// Raising nice is open to every user; the step is taken from whatever the
/// suite runs at.
#[test]
fn a_renice_of_the_same_process_is_applied() {
    let child = Sleeper::spawn();
    let Some(later) = nice_of(&child).step(NiceStep::Lower) else {
        return;
    };
    let result = control().set_priority(key_of(&child), PriorityChange::Nice(later));
    assert_eq!(result, Ok(()));
    assert_eq!(nice_of(&child), later);
}

#[test]
fn a_renice_with_a_wrong_start_changes_nothing() {
    let child = Sleeper::spawn();
    let before = nice_of(&child);
    let result = control().set_priority(successor_of(&child), PriorityChange::Nice(Niceness::MAX));
    assert_eq!(result, Err(ControlError::Replaced));
    assert_eq!(nice_of(&child), before);
}

#[test]
fn a_priority_class_is_not_changed_on_linux() {
    let child = Sleeper::spawn();
    let class = PriorityChange::Class(SettableClass::Normal);
    assert_eq!(
        control().set_priority(key_of(&child), class),
        Err(ControlError::Unsupported)
    );
}
