//! Purpose: Tests for the process wrappers against the real kernel.
//!
//! Every signal goes to a `sleep` child the test spawned itself while it
//! still holds its PID, or to a PID no process can hold; never to a reaped
//! child's PID, which the kernel may already have handed to a stranger. Every
//! value is read back from the kernel rather than trusted from the call.

use std::os::unix::process::ExitStatusExt;

#[cfg(target_os = "linux")]
use crate::models::NiceStep;
use crate::models::{Niceness, Pid, Signal};
use crate::platform::{set_niceness, signal_process};
use crate::test_support::sleeper::Sleeper;

/// A PID no process can hold: above Linux's `PID_MAX_LIMIT` (4 194 304) and
/// the BSDs' and macOS's 99 999. A call to it answers `ESRCH` with no race,
/// where a reaped child's PID may already belong to someone else.
fn absent_pid() -> Pid {
    Pid::new(i32::MAX.unsigned_abs()).expect("i32::MAX is a PID")
}

/// The unpinned path: plain `kill(2)` on one PID.
#[test]
fn signal_process_ends_a_child_with_the_signal_sent() {
    let mut child = Sleeper::spawn();
    signal_process(child.pid(), Signal::Terminate).expect("signal");
    assert_eq!(child.wait().signal(), Some(libc::SIGTERM));
}

/// The kernel's own ceiling on the PIDs it hands out, `pid_max`, is below
/// [`absent_pid`], so the `ESRCH` tests can never reach a process.
#[cfg(target_os = "linux")]
#[test]
fn no_process_can_hold_the_absent_pid() {
    let pid_max: i32 = std::fs::read_to_string("/proc/sys/kernel/pid_max")
        .expect("pid_max")
        .trim()
        .parse()
        .expect("a numeric pid_max");
    assert!(absent_pid().raw() > pid_max);
}

#[test]
fn signal_process_to_a_pid_no_process_holds_is_esrch() {
    let error = signal_process(absent_pid(), Signal::Terminate).expect_err("no such process");
    assert_eq!(error.raw_os_error(), Some(libc::ESRCH));
}

/// `/proc/<pid>/stat` field 19, the nice value the kernel holds.
#[cfg(target_os = "linux")]
fn nice_of(pid: Pid) -> Niceness {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).expect("stat");
    let (_, tail) = stat.rsplit_once(')').expect("a closing parenthesis");
    let nice: i32 = tail
        .split_whitespace()
        .nth(19 - 3)
        .expect("a nice field")
        .parse()
        .expect("a numeric nice");
    Niceness::try_from(nice).expect("a nice value")
}

/// The kernel's `can_nice` rule: lowering nice to `target` needs
/// `CAP_SYS_NICE`, or an `RLIMIT_NICE` of at least `19 - target + 1`, the
/// kernel's `nice_to_rlimit`. The child inherited this process's limit.
#[cfg(target_os = "linux")]
fn may_lower_nice_to(target: Niceness) -> bool {
    let needed = u64::try_from(Niceness::MAX.get() - target.get() + 1).expect("1..=40");
    let limit = rustix::process::getrlimit(rustix::process::Resource::Nice).current;
    limit.is_none_or(|limit| limit >= needed) || has_cap_sys_nice()
}

/// Whether this thread holds `CAP_SYS_NICE` where the kernel checks it: in
/// the initial user namespace, whose `uid_map` maps every uid to itself
/// (`0 0 4294967295`).
#[cfg(target_os = "linux")]
fn has_cap_sys_nice() -> bool {
    const CAP_SYS_NICE: u32 = 23;
    let uid_map = std::fs::read_to_string("/proc/self/uid_map").expect("uid_map");
    let initial_namespace = uid_map.split_whitespace().eq(["0", "0", "4294967295"]);
    let status = std::fs::read_to_string("/proc/thread-self/status").expect("status");
    let effective = status
        .lines()
        .find_map(|line| line.strip_prefix("CapEff:"))
        .map(|hex| u64::from_str_radix(hex.trim(), 16).expect("a hex capability set"))
        .expect("a CapEff line");
    initial_namespace && effective & (1 << CAP_SYS_NICE) != 0
}

/// Raising nice is open to every user. The step is taken from the nice the
/// child inherited, which is whatever the suite runs at.
#[cfg(target_os = "linux")]
#[test]
fn set_niceness_changes_what_the_kernel_reports() {
    let child = Sleeper::spawn();
    let Some(later) = nice_of(child.pid()).step(NiceStep::Lower) else {
        eprintln!("skipped: the suite runs at nice 19, so nothing is higher");
        return;
    };
    set_niceness(child.pid(), later).expect("raise nice");
    assert_eq!(nice_of(child.pid()), later);
}

/// Whether lowering nice is allowed depends on who runs the suite, so the
/// kernel's own rule says which answer to expect; both answers are asserted.
#[cfg(target_os = "linux")]
#[test]
fn lowering_nice_follows_the_kernels_rule() {
    let child = Sleeper::spawn();
    let start = nice_of(child.pid());
    let Some(sooner) = start.step(NiceStep::Raise) else {
        eprintln!("skipped: the suite runs at nice -20, so nothing is lower");
        return;
    };
    let answer = set_niceness(child.pid(), sooner);
    if may_lower_nice_to(sooner) {
        answer.expect("the kernel allows this nice value");
        assert_eq!(nice_of(child.pid()), sooner);
    } else {
        let error = answer.expect_err("the kernel refuses this nice value");
        assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
        assert_eq!(nice_of(child.pid()), start);
    }
}

#[test]
fn set_niceness_on_a_pid_no_process_holds_is_esrch() {
    let error = set_niceness(absent_pid(), Niceness::MAX).expect_err("no such process");
    assert_eq!(error.raw_os_error(), Some(libc::ESRCH));
}

#[cfg(target_os = "linux")]
#[test]
fn a_process_handle_signals_the_process_it_pinned() {
    let mut child = Sleeper::spawn();
    let handle = crate::platform::ProcessHandle::open(child.pid()).expect("pidfd");
    handle.signal(Signal::Kill).expect("send");
    assert_eq!(child.wait().signal(), Some(libc::SIGKILL));
}

/// The pidfd outlives the process it names: once that process is reaped the
/// kernel answers `ESRCH`, whoever holds the PID by then. The signal goes
/// through the pidfd, never to the freed PID.
///
/// A live child or an unreaped zombie would take the signal, so this also
/// proves that dropping a [`Sleeper`] kills and reaps it.
#[cfg(target_os = "linux")]
#[test]
fn a_handle_to_a_reaped_process_answers_esrch() {
    let child = Sleeper::spawn();
    let handle = crate::platform::ProcessHandle::open(child.pid()).expect("pidfd");
    drop(child);
    let error = handle.signal(Signal::Terminate).expect_err("gone");
    assert_eq!(error.raw_os_error(), Some(libc::ESRCH));
}

#[cfg(target_os = "linux")]
#[test]
fn no_handle_pins_a_pid_no_process_holds() {
    let error = crate::platform::ProcessHandle::open(absent_pid()).expect_err("nothing to pin");
    assert_eq!(error.raw_os_error(), Some(libc::ESRCH));
}
