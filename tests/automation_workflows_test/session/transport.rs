//! Purpose: Read complete protocol frames and prove graceful shutdown without a forced kill.
#![expect(
    clippy::unwrap_used,
    reason = "integration-test helper: panics on fixture failure are correct behavior"
)]

use std::{
    io::{BufRead, BufReader},
    process::{Child, ChildStdout},
    sync::mpsc::{self, Receiver},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

pub(super) fn reader(stdout: ChildStdout) -> (Receiver<String>, JoinHandle<()>) {
    let (send, receive) = mpsc::sync_channel(8);
    let reader = thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let Ok(line) = line else { break };
            if send.send(line).is_err() {
                break;
            }
        }
    });
    (receive, reader)
}

pub(super) fn wait_for_exit(child: &mut Child) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(status.success(), "automation process exited with {status}");
            return;
        }
        assert!(
            Instant::now() < deadline,
            "automation process did not exit after closing stdin"
        );
        thread::sleep(Duration::from_millis(10));
    }
}
