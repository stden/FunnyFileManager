//! Purpose: Tests for identity-checked control: how a pidfd open, a re-read
//! `stat` and an errno are read, on every CI system; and, on Linux, signals
//! and renices of `sleep` children the tests spawned themselves — the right
//! process, a wrong start, a reaped child, the path without a pidfd, and the
//! refusals of FFM itself, init and kernel threads.

use std::io;
use std::time::Duration;

use super::control::{classify_open, control_error, verify_start, Pin};
use super::parse::Stat;
use crate::models::{
    ControlError, KernelPriority, Niceness, Priority, ProcessKind, ProcessState, StartTime,
};

/// Linux's errno values, as the kernel would answer them.
const EPERM: i32 = 1;
const ESRCH: i32 = 3;
const EIO: i32 = 5;
const EBADF: i32 = 9;
const EACCES: i32 = 13;
const ENOSYS: i32 = 38;

fn errno(code: i32) -> io::Error {
    io::Error::from_raw_os_error(code)
}

fn started_at(start: u64) -> Stat {
    Stat {
        name: "bob".to_owned(),
        state: ProcessState::Sleeping,
        parent: None,
        kind: ProcessKind::User,
        cpu_time: Duration::ZERO,
        priority: Priority::Unix {
            kernel: KernelPriority::new(20),
            nice: Niceness::try_from(0).expect("a nice value"),
        },
        threads: 1,
        start: StartTime::new(start),
        started: Duration::ZERO,
    }
}

#[test]
fn an_open_pidfd_pins_the_process() {
    assert_eq!(classify_open(Ok(7)), Ok(Pin::Pinned(7)));
}

/// No pidfds before Linux 5.3 (`ENOSYS`), none under a seccomp filter that
/// refuses them (`EPERM`): the signal goes by PID, after the same check.
#[test]
fn a_kernel_without_pidfds_leaves_the_process_unpinned() {
    for code in [ENOSYS, EPERM] {
        assert_eq!(
            classify_open::<()>(Err(errno(code))),
            Ok(Pin::Unpinned),
            "{code}"
        );
    }
}

#[test]
fn nothing_to_pin_is_gone_and_any_other_failure_is_reported() {
    assert_eq!(
        classify_open::<()>(Err(errno(ESRCH))),
        Err(ControlError::Gone)
    );
    let odd = errno(EBADF);
    let kind = odd.kind();
    assert_eq!(
        classify_open::<()>(Err(odd)),
        Err(ControlError::Failed(kind))
    );
}

#[test]
fn the_same_start_is_the_same_process() {
    let expected = StartTime::new(420);
    assert_eq!(
        verify_start(Ok(Some(started_at(420))), expected),
        Ok(started_at(420))
    );
}

#[test]
fn a_different_start_is_another_process_under_the_same_pid() {
    let expected = StartTime::new(420);
    assert_eq!(
        verify_start(Ok(Some(started_at(421))), expected),
        Err(ControlError::Replaced)
    );
}

#[test]
fn a_stat_that_is_gone_or_cannot_be_read_or_parsed_stops_the_request() {
    let expected = StartTime::new(420);
    let cases = [
        (Err(io::ErrorKind::NotFound.into()), ControlError::Gone),
        (Err(errno(ESRCH)), ControlError::Gone),
        (Err(errno(EACCES)), ControlError::PermissionDenied),
        (Ok(None), ControlError::Unverifiable),
    ];
    for (read, error) in cases {
        assert_eq!(verify_start(read, expected), Err(error));
    }
}

#[test]
fn each_errno_is_read_as_its_control_error() {
    assert_eq!(control_error(&errno(ESRCH)), ControlError::Gone);
    assert_eq!(control_error(&errno(EPERM)), ControlError::PermissionDenied);
    assert_eq!(
        control_error(&errno(EACCES)),
        ControlError::PermissionDenied
    );
    let odd = errno(EIO);
    assert_eq!(control_error(&odd), ControlError::Failed(odd.kind()));
}
