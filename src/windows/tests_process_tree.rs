//! Purpose: Tests for the job object that owns a child's process tree.

use super::process_tree_job;
use std::process::Command;

#[cfg(windows)]
#[test]
fn dropping_the_process_tree_job_terminates_its_owned_process() {
    use std::os::windows::process::CommandExt;
    use std::time::{Duration, Instant};
    let root = tempfile::TempDir::new().unwrap();
    let mut child = Command::new("powershell.exe")
        .args(["-NoProfile", "-Command", "Start-Sleep -Seconds 30"])
        .current_dir(root.path())
        .creation_flags(0x0800_0000)
        .spawn()
        .unwrap();
    let job = process_tree_job(&child).unwrap();
    assert!(child.try_wait().unwrap().is_none());
    drop(job);
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline && child.try_wait().unwrap().is_none() {
        std::thread::sleep(Duration::from_millis(10));
    }
    let exited = child.try_wait().unwrap().is_some();
    if !exited {
        child.kill().unwrap();
    }
    child.wait().unwrap();
    assert!(exited, "the job must not leave its server running");
}
