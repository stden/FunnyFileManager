//! Purpose: The refusals of identity-checked control, against the real
//! kernel. Every test is built so that, were the refusal missing, nothing
//! would reach a process the test does not own: init is asked for with a
//! start it cannot have, FFM itself is a child standing in for it, and a
//! kernel thread is a child whose `stat` a fake `/proc` rewrites.

use std::fs;

use tempfile::TempDir;

use super::control::ProcessControl;
use super::parse;
use super::tests_control_kernel::{control, control_by, hertz, key_of, pid, stat_line, GRACE};
use crate::models::{ControlError, ControlRefusal, ProcessKey, Signal, Start, StartTime};
use crate::test_support::sleeper::Sleeper;

/// `PF_KTHREAD` among the flags of a `stat` line.
const KERNEL_THREAD_FLAGS: &str = "2129984";

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
