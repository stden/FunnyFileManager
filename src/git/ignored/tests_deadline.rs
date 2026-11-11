//! Purpose: A `git check-ignore` that runs past its deadline is stopped and
//! reaped, not left running behind the listing.

use std::process::{Command, Stdio};
use std::time::Duration;

use super::wait_for_exit;

#[test]
fn an_ignore_probe_that_exceeds_its_deadline_is_stopped() {
    let mut child = Command::new("sh")
        .args(["-c", "exec sleep 10"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    assert_eq!(wait_for_exit(&mut child, Duration::ZERO), None);
    assert!(child.try_wait().unwrap().is_some());
}
