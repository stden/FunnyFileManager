//! Purpose: The one kind of real process a test may signal or renice: a
//! short-lived sleeper child it spawned itself, killed and reaped when dropped.

#[cfg(windows)]
use std::process::Stdio;
use std::process::{Child, Command, ExitStatus};
#[cfg(target_os = "linux")]
use std::time::{Duration, Instant};

use crate::models::Pid;

#[cfg(target_os = "linux")]
const POLL: Duration = Duration::from_millis(10);

/// A child process the test spawned, killed and reaped when dropped, so a
/// failed assertion leaves no process behind.
pub struct Sleeper(Child);

impl Sleeper {
    #[cfg(not(windows))]
    pub fn spawn() -> Self {
        Self(
            Command::new("sleep")
                .arg("30")
                .spawn()
                .expect("spawn sleep"),
        )
    }

    #[cfg(windows)]
    pub fn spawn() -> Self {
        Self(
            Command::new("ping")
                .args(["-n", "31", "127.0.0.1"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("spawn sleeper ping"),
        )
    }

    pub fn pid(&self) -> Pid {
        Pid::new(self.0.id()).expect("a child's PID is in range")
    }

    /// Waits for the child to end, reaps it and says how it ended.
    pub fn wait(&mut self) -> ExitStatus {
        self.0.wait().expect("reap")
    }

    #[cfg(target_os = "linux")]
    pub fn outlives(&mut self, grace: Duration) -> bool {
        let deadline = Instant::now() + grace;
        while Instant::now() < deadline {
            if self.0.try_wait().expect("poll the child").is_some() {
                return false;
            }
            std::thread::sleep(POLL);
        }
        self.0.try_wait().expect("poll the child").is_none()
    }
}

impl Drop for Sleeper {
    fn drop(&mut self) {
        drop(self.0.kill());
        drop(self.0.wait());
    }
}
