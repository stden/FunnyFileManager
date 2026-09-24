//! Purpose: run the real `ffm` binary on a pseudo-terminal, where the input
//! loop meets crossterm's own event queue and a terminal that really hangs up.
//!
//! Every other test injects events and state into the loop. What lives
//! between the kernel's pty and crossterm's reader, and what only the
//! program's own start-up hands the app, can be seen only in the binary on a
//! pty. Linux only: the hang-up is the one Linux reports, where `isatty` on
//! the hung-up terminal fails with `EIO`.
#![cfg(target_os = "linux")]

use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use rustix::process::{kill_process_group, test_kill_process_group, Pid, Signal};
use rustix::pty::{grantpt, openpt, ptsname, unlockpt, OpenptFlags};
use rustix::termios::{tcsetwinsize, Winsize};
use tempfile::TempDir;

#[path = "pty_session_test/ffm_process.rs"]
mod ffm_process;
#[path = "pty_session_test/hang_up.rs"]
mod hang_up;
#[path = "pty_session_test/input.rs"]
mod input;
use ffm_process::Ffm;

/// Far more than a burst needs, so a failure reports how long it really took.
const PATIENCE: Duration = Duration::from_secs(30);
/// How soon a hung-up terminal must end ffm and whatever it started.
const HANG_UP: Duration = Duration::from_secs(5);

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}
