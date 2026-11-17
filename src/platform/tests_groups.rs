//! Purpose: Tests for hanging up a process group and for the exit check that
//! leaves the leader unreaped, against the real kernel.

use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

use crate::platform::{hang_up_process_group, has_exited};

/// `sleep` leading a process group of its own.
fn group_leader() -> Child {
    Command::new("sleep")
        .arg("30")
        .process_group(0)
        .spawn()
        .expect("spawn sleep")
}

/// Poll [`has_exited`] until it answers yes, for at most a few seconds.
fn exited_soon(child: &Child) -> bool {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if has_exited(child).expect("waitid") {
            return true;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    false
}

#[test]
fn hanging_up_a_group_ends_its_leader_with_sighup() {
    let mut leader = group_leader();
    hang_up_process_group(&leader).expect("hang up the group");
    assert_eq!(leader.wait().expect("reap").signal(), Some(libc::SIGHUP));
}

/// A child left in this process's group leads no group of its own: the call
/// must fail rather than hang up the test runner.
#[test]
fn a_child_that_leads_no_group_is_not_hung_up() {
    let mut child = Command::new("sleep").arg("30").spawn().expect("spawn");
    let error = hang_up_process_group(&child).expect_err("no such group");
    assert_eq!(error.raw_os_error(), Some(libc::ESRCH));
    child.kill().expect("cleanup");
    child.wait().expect("reap");
}

#[test]
fn a_running_child_has_not_exited() {
    let mut child = group_leader();
    assert!(!has_exited(&child).expect("waitid"));
    child.kill().expect("cleanup");
    child.wait().expect("reap");
}

/// The answer leaves the child a zombie: its status is still there for
/// `wait`, so its id still names its group until then.
#[test]
fn an_exited_child_is_reported_and_left_to_reap() {
    let mut child = group_leader();
    child.kill().expect("kill");
    assert!(exited_soon(&child), "the killed child never exited");
    assert!(has_exited(&child).expect("still unreaped"));
    assert_eq!(child.wait().expect("reap").signal(), Some(libc::SIGKILL));
}
