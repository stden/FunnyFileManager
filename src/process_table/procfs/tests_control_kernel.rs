//! Purpose: Identity-checked control against the real kernel. Every signal
//! and renice is aimed at a `sleep` child the test spawned and still holds,
//! and every refusal test is built so that, were the refusal missing,
//! nothing would reach a process the test does not own: init is asked for
//! with a start it cannot have, FFM itself is a child standing in for it,
//! and a kernel thread is a child whose `stat` a fake `/proc` rewrites.

use std::fs;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::time::Duration;

use tempfile::TempDir;

use super::control::{send, Pin, ProcessControl};
use super::parse::{self, ClockTicks};
use crate::models::{
    ControlError, ControlRefusal, NiceStep, Niceness, Pid, Priority, PriorityChange, ProcessKey,
    SettableClass, Signal, Start, StartTime,
};
use crate::test_support::sleeper::Sleeper;

/// Long enough for a signal that ends a process to have ended the child.
const GRACE: Duration = Duration::from_millis(200);

/// `PF_KTHREAD` among the flags of a `stat` line.
const KERNEL_THREAD_FLAGS: &str = "2129984";

fn hertz() -> ClockTicks {
    ClockTicks::new(100).expect("USER_HZ is not zero")
}

fn pid(raw: u32) -> Pid {
    Pid::new(raw).expect("a PID")
}

/// Control of the real `/proc`, run from `own`.
fn control_by(own: Pid) -> ProcessControl {
    ProcessControl::new(PathBuf::from("/proc"), own, hertz())
}

/// Control of the real `/proc`, run from this test process.
fn control() -> ProcessControl {
    control_by(pid(std::process::id()))
}

fn stat_line(child: &Sleeper) -> String {
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
fn key_of(child: &Sleeper) -> ProcessKey {
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

/// A child stands in for FFM, so a missing refusal would end only it.
#[test]
fn ffm_itself_is_refused_before_anything_is_sent() {
    let mut child = Sleeper::spawn();
    let result = control_by(child.pid()).signal(key_of(&child), Signal::Terminate);
    assert_eq!(
        result,
        Err(ControlError::Refused(ControlRefusal::OwnProcess))
    );
    assert!(child.outlives(GRACE));
}

/// Asked with a start init cannot have: were the refusal missing, the
/// re-read would find another start and nothing would be sent either.
#[test]
fn init_is_refused_before_anything_is_sent() {
    let impossible = ProcessKey::new(pid(1), Start::Known(StartTime::new(u64::MAX)));
    let result = control().signal(impossible, Signal::Interrupt);
    assert_eq!(result, Err(ControlError::Refused(ControlRefusal::Init)));
}

/// The pidfd pins the real child, and a fake `/proc` says it is a kernel
/// thread: a missing refusal would end only the child.
#[test]
fn a_kernel_thread_is_refused_once_its_stat_is_read_again() {
    let mut child = Sleeper::spawn();
    let fake = TempDir::new().expect("a fake /proc");
    let directory = fake.path().join(child.pid().to_string());
    fs::create_dir(&directory).expect("a process directory");
    let line = stat_line(&child);
    let (head, tail) = line.rsplit_once(") ").expect("a stat line");
    let mut fields: Vec<&str> = tail.split(' ').collect();
    fields[9 - 3] = KERNEL_THREAD_FLAGS;
    fs::write(
        directory.join("stat"),
        format!("{head}) {}", fields.join(" ")),
    )
    .expect("stat");
    let control = ProcessControl::new(fake.path().to_path_buf(), pid(std::process::id()), hertz());
    let result = control.signal(key_of(&child), Signal::Kill);
    assert_eq!(
        result,
        Err(ControlError::Refused(ControlRefusal::KernelThread))
    );
    assert!(child.outlives(GRACE));
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

/// The system's control is run by this process: asked to continue this
/// process, which would be harmless, it refuses.
#[test]
fn the_system_control_refuses_to_signal_this_process() {
    let control = ProcessControl::system().expect("Linux reports a PID and a clock rate");
    let own = pid(std::process::id());
    let result = control.signal(ProcessKey::new(own, Start::Unknown), Signal::Continue);
    assert_eq!(result, Err(ControlError::Unverifiable));
    let own_stat = fs::read_to_string("/proc/self/stat").expect("own stat");
    let stat = parse::stat(&own_stat, own, hertz()).expect("own stat line");
    let key = ProcessKey::new(own, Start::Known(stat.start));
    let result = control.signal(key, Signal::Continue);
    assert_eq!(
        result,
        Err(ControlError::Refused(ControlRefusal::OwnProcess))
    );
}
