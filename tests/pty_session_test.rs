//! Purpose: run the real `ffm` binary on a pseudo-terminal, where the input
//! loop meets crossterm's own event queue and a terminal that really hangs up.
//!
//! Every other test injects events into the loop. These two behaviours live
//! between the kernel's pty and crossterm's reader, so only the binary on a
//! pty can show them. Linux only: the hang-up is the one Linux reports, where
//! `isatty` on the hung-up terminal fails with `EIO`.
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

/// The first frame of a debug build, on a machine busy with the rest of the suite.
const STARTUP: Duration = Duration::from_secs(60);
/// How often the test looks at the screen, the files and the process.
const TICK: Duration = Duration::from_millis(5);
/// Far more than a burst needs, so a failure reports how long it really took.
const PATIENCE: Duration = Duration::from_secs(30);
/// How soon a hung-up terminal must end ffm and whatever it started.
const HANG_UP: Duration = Duration::from_secs(5);

/// `ffm` on its own pseudo-terminal, in a sandbox home and working directory.
struct Ffm {
    /// The terminal's far end. Dropping it hangs the terminal up.
    terminal: Option<File>,
    process: Child,
    screen: Vec<u8>,
    sandbox: TempDir,
}

impl Ffm {
    fn start() -> Self {
        let sandbox = TempDir::new().expect("sandbox");
        let home = sandbox.path().join("home");
        fs::create_dir_all(&home).expect("home");
        fs::create_dir_all(sandbox.path().join("work")).expect("work");
        let (terminal, device) = open_pty();
        let process = Command::new(env!("CARGO_BIN_EXE_ffm"))
            .current_dir(sandbox.path().join("work"))
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap_or_default())
            .env("HOME", &home)
            .env("TERM", "xterm-256color")
            .env("LANG", "C.UTF-8")
            .stdin(Stdio::from(device.try_clone().expect("terminal for stdin")))
            .stdout(Stdio::from(
                device.try_clone().expect("terminal for stdout"),
            ))
            .stderr(Stdio::from(device))
            .spawn()
            .expect("start ffm");
        let mut ffm = Self {
            terminal: Some(terminal),
            process,
            screen: Vec::new(),
            sandbox,
        };
        let drawn = ffm.wait_for(STARTUP, |ffm| contains(&ffm.screen, b"Quit"));
        assert!(drawn.is_some(), "ffm never drew its first frame");
        ffm
    }

    fn work(&self) -> PathBuf {
        self.sandbox.path().join("work")
    }

    /// Send `keys` in one write, the way a paste or a fast typist's burst lands.
    fn type_keys(&mut self, keys: &[u8]) {
        let terminal = self.terminal.as_mut().expect("the terminal is up");
        terminal.write_all(keys).expect("type");
    }

    fn hang_up(&mut self) {
        self.terminal = None;
    }

    /// How ffm ended, or `None` if it is still running after [`HANG_UP`].
    fn wait_for_exit(&mut self) -> Option<ExitStatus> {
        self.wait_for(HANG_UP, |ffm| {
            ffm.process.try_wait().expect("ffm status").is_some()
        })?;
        self.process.try_wait().expect("ffm status")
    }

    /// Poll until `done`, reading the screen so ffm never blocks on a full
    /// terminal. The time it took, or `None` past `limit`.
    fn wait_for(
        &mut self,
        limit: Duration,
        mut done: impl FnMut(&mut Self) -> bool,
    ) -> Option<Duration> {
        let started = Instant::now();
        loop {
            self.read_screen();
            if done(self) {
                return Some(started.elapsed());
            }
            if started.elapsed() > limit {
                return None;
            }
            std::thread::sleep(TICK);
        }
    }

    fn read_screen(&mut self) {
        let Some(terminal) = self.terminal.as_mut() else {
            return;
        };
        let mut chunk = [0_u8; 4096];
        // `WouldBlock` is an empty terminal; `EIO` is one ffm has let go of.
        while let Ok(read @ 1..) = terminal.read(&mut chunk) {
            self.screen.extend_from_slice(&chunk[..read]);
        }
    }
}

impl Drop for Ffm {
    fn drop(&mut self) {
        // A failed test leaves ffm running; it must not outlive the test.
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

/// A pty pair: the nonblocking far end the test holds, and the terminal
/// device ffm runs on.
fn open_pty() -> (File, File) {
    let far_end =
        openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY | OpenptFlags::CLOEXEC).expect("open a pty");
    grantpt(&far_end).expect("grant the pty");
    unlockpt(&far_end).expect("unlock the pty");
    let size = Winsize {
        ws_row: 30,
        ws_col: 100,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    tcsetwinsize(&far_end, size).expect("size the pty");
    rustix::io::ioctl_fionbio(&far_end, true).expect("nonblocking pty");
    let name = ptsname(&far_end, Vec::new()).expect("name the pty");
    let device = OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NOCTTY)
        .open(OsStr::from_bytes(name.as_bytes()))
        .expect("open the terminal device");
    (File::from(far_end), device)
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

/// A burst lands in one read, so crossterm parses all of it at once and keeps
/// the rest in its own queue, where no wait on the terminal can see it. Each
/// queued key used to cost the whole 100 ms idle poll: this line took about
/// eight seconds to arrive, one character every tenth of a second.
#[test]
fn a_burst_of_typed_keys_is_handled_without_waiting_between_keys() {
    let mut ffm = Ffm::start();
    let name = "abcdefghijklmnopqrstuvwxyz0123456789".repeat(2);

    ffm.type_keys(format!(":mkdir {name}\r").as_bytes());
    let folder = ffm.work().join(&name);
    let took = ffm.wait_for(PATIENCE, |_| folder.is_dir());

    let took = took.expect("the typed command never ran");
    assert!(
        took < Duration::from_secs(3),
        "80 queued keys took {took:?}; they must not wait for each other"
    );
}

/// Hanging up while a `:` command runs: `isatty` on a hung-up terminal fails,
/// which used to read as "not a terminal, leave it to crossterm". crossterm
/// then read the dead terminal forever at 100% CPU, the command's timeout
/// never came round, and its process group outlived ffm.
#[test]
fn a_terminal_that_hangs_up_during_a_shell_command_stops_it_and_ends_ffm() {
    let mut ffm = Ffm::start();
    let pid_file = ffm.work().join("shell.pid");
    ffm.type_keys(b":echo $$ > shell.pid; sleep 30\r");
    let started = ffm.wait_for(PATIENCE, |_| read_group(&pid_file).is_some());
    let group = started
        .and_then(|_| read_group(&pid_file))
        .expect("the shell never started");

    ffm.hang_up();
    let status = ffm.wait_for_exit();
    let group_gone = ffm.wait_for(HANG_UP, |_| test_kill_process_group(group).is_err());
    // A failed run must not leave the command behind either.
    let _ = kill_process_group(group, Signal::KILL);

    assert_ended_cleanly(status);
    assert!(group_gone.is_some(), "the shell command outlived ffm");
}

/// Waiting for a key, ffm sits in `poll(2)`, which sees the hang-up. What
/// followed was the failure: restoring the dead terminal fails, and printing
/// that failure onto it panicked.
#[test]
fn a_terminal_that_hangs_up_while_ffm_waits_for_a_key_ends_it_cleanly() {
    let mut ffm = Ffm::start();

    ffm.hang_up();

    assert_ended_cleanly(ffm.wait_for_exit());
}

fn assert_ended_cleanly(status: Option<ExitStatus>) {
    let status = status.expect("ffm kept running after its terminal hung up");
    assert!(
        status.success(),
        "a hang-up ends the session the way F10 does, not as a failure: {status}"
    );
}

/// The process group the shell wrote down: `$$` of `sh -c`, which ffm starts
/// as the leader of its own group.
fn read_group(path: &Path) -> Option<Pid> {
    let text = fs::read_to_string(path).ok()?;
    Pid::from_raw(text.trim().parse().ok()?)
}
