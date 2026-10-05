//! Purpose: The one kind of real process a test may signal or renice: a
//! `sleep 30` child it spawned itself, killed and reaped when dropped.
//!
//! Until it is reaped the child holds its PID, even as a zombie, so nothing
//! sent to that PID can reach anyone else. A test never aims a raw call at a
//! reaped child's PID, which the kernel may already have handed to a
//! stranger.

use std::process::{Child, Command, ExitStatus};
#[cfg(target_os = "linux")]
use std::time::{Duration, Instant};

use crate::models::Pid;

/// How often [`Sleeper::outlives`] looks.
#[cfg(target_os = "linux")]
const POLL: Duration = Duration::from_millis(10);

/// A `sleep 30` child the test spawned, killed and reaped when dropped, so a
/// failed assertion leaves no process behind.
///
/// Once [`Sleeper::wait`] has reaped it, std's `kill` sends nothing.
pub struct Sleeper(Child);

impl Sleeper {
    pub fn spawn() -> Self {
        Self(
            Command::new("sleep")
                .arg("30")
                .spawn()
                .expect("spawn sleep"),
        )
    }

    pub fn pid(&self) -> Pid {
        Pid::new(self.0.id()).expect("a child's PID is in range")
    }

    /// Waits for the child to end, reaps it and says how it ended.
    pub fn wait(&mut self) -> ExitStatus {
        self.0.wait().expect("reap")
    }

    /// Whether the child is still running once `grace` has passed: a signal
    /// that ends a process, sent just before, has ended it by then. The
    /// child sleeps for 30 s, so one nothing reached always outlives it.
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
        // Neither error can be acted on: `kill` fails only for a child that
        // has exited already, and a panic in `drop` would abort the run.
        drop(self.0.kill());
        drop(self.0.wait());
    }
}
