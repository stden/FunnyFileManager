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

struct PrivateProbe(std::process::Child);

impl Drop for PrivateProbe {
    fn drop(&mut self) {
        // The process may already have exited; still reap it after every failure path.
        drop(self.0.kill());
        drop(self.0.wait());
    }
}

fn private_probe(directory: &std::path::Path) -> PrivateProbe {
    use std::{os::windows::process::CommandExt, process::Stdio};
    PrivateProbe(
        Command::new("powershell.exe")
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Start-Sleep -Seconds 30",
            ])
            .current_dir(directory)
            .creation_flags(0x0800_0000)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    )
}

#[test]
fn private_terminal_job_owns_a_borrowed_portable_child_without_taking_its_handle() {
    use std::time::{Duration, Instant};
    let root = tempfile::TempDir::new().unwrap();
    let mut child = private_probe(root.path());
    let job = super::private_terminal_job(&child.0).unwrap();
    assert!(child.0.try_wait().unwrap().is_none());
    drop(job);
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline && child.0.try_wait().unwrap().is_none() {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        child.0.try_wait().unwrap().is_some(),
        "private job left its owned child alive"
    );
}
