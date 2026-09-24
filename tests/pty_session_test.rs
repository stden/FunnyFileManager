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
    #[expect(
        clippy::expect_used,
        reason = "test fixture: an error from the pty or the ffm process fails the test driving it"
    )]
    fn start() -> Self {
        let path = std::env::var_os("PATH").unwrap_or_default();
        Self::start_in(TempDir::new().expect("sandbox"), &path)
    }

    /// `ffm` with `sandbox/home` as its home, `sandbox/work` as its working
    /// directory and `path` as its `PATH`; whatever the sandbox holds already
    /// is there when it starts.
    #[expect(
        clippy::expect_used,
        reason = "test fixture: an error from the pty or the ffm process fails the test driving it"
    )]
    fn start_in(sandbox: TempDir, path: &OsStr) -> Self {
        let home = sandbox.path().join("home");
        fs::create_dir_all(&home).expect("home");
        fs::create_dir_all(sandbox.path().join("work")).expect("work");
        let (terminal, device) = open_pty();
        let process = Command::new(env!("CARGO_BIN_EXE_ffm"))
            .current_dir(sandbox.path().join("work"))
            .env_clear()
            .env("PATH", path)
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
        ffm.wait_for_first_frame();
        ffm
    }

    /// Wait for the first frame; an ffm that ends at once fails here, not
    /// when [`STARTUP`] runs out.
    #[expect(
        clippy::expect_used,
        reason = "test fixture: an error from the pty or the ffm process fails the test driving it"
    )]
    fn wait_for_first_frame(&mut self) {
        let drawn = self.wait_for(STARTUP, |ffm| {
            contains(&ffm.screen, b"Quit") || ffm.process.try_wait().expect("ffm status").is_some()
        });
        assert!(drawn.is_some(), "ffm never drew its first frame");
        let ended = self.process.try_wait().expect("ffm status");
        assert!(
            ended.is_none(),
            "ffm ended before its first frame: {ended:?}"
        );
    }

    fn work(&self) -> PathBuf {
        self.sandbox.path().join("work")
    }

    /// Send `keys` in one write, the way a paste or a fast typist's burst lands.
    #[expect(
        clippy::expect_used,
        reason = "test fixture: an error from the pty or the ffm process fails the test driving it"
    )]
    fn type_keys(&mut self, keys: &[u8]) {
        let terminal = self.terminal.as_mut().expect("the terminal is up");
        terminal.write_all(keys).expect("type");
    }

    fn hang_up(&mut self) {
        self.terminal = None;
    }

    /// How ffm ended, or `None` if it is still running after [`HANG_UP`].
    #[expect(
        clippy::expect_used,
        reason = "test fixture: an error from the pty or the ffm process fails the test driving it"
    )]
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
            self.screen.extend(chunk.iter().take(read));
        }
    }
}

impl Drop for Ffm {
    fn drop(&mut self) {
        // A failed test leaves ffm running; it must not outlive the test. Best
        // effort: a panic here, while a failed test unwinds, would abort the run.
        drop(self.process.kill());
        drop(self.process.wait());
    }
}

/// A pty pair: the nonblocking far end the test holds, and the terminal
/// device ffm runs on.
#[expect(
    clippy::expect_used,
    reason = "test fixture: an error from the pty or the ffm process fails the test driving it"
)]
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

/// The Language servers dialog says what this machine would start, so the
/// program has to hand it the `PATH` it runs with: a server whose command is
/// there is found. Every other test gives the dialog a fake machine.
#[test]
fn the_language_servers_dialog_finds_a_server_on_the_path_ffm_runs_with() {
    use std::os::unix::fs::PermissionsExt as _;

    let sandbox = TempDir::new().expect("sandbox");
    let bin = sandbox.path().join("bin");
    fs::create_dir_all(&bin).expect("bin");
    let server = bin.join("aaa-fixture-server");
    fs::write(&server, "#!/bin/sh\n").expect("server");
    fs::set_permissions(&server, fs::Permissions::from_mode(0o755)).expect("chmod");
    let config = sandbox.path().join("home/.config/ffm");
    fs::create_dir_all(&config).expect("config");
    fs::write(
        config.join("settings.yaml"),
        "lsp:\n  aaa: {command: aaa-fixture-server, extensions: [aaafixture], install: null}\n",
    )
    .expect("settings");
    let inherited = std::env::var_os("PATH").unwrap_or_default();
    let path = std::env::join_paths(std::iter::once(bin).chain(std::env::split_paths(&inherited)))
        .expect("PATH");
    let mut ffm = Ffm::start_in(sandbox, &path);

    ffm.type_keys(b":lsp\r");
    // The status is checked before the dialog is drawn, so the first frame
    // that shows aaa's status shows the answer.
    let found = b"found configured executable";
    let shown = ffm.wait_for(PATIENCE, |ffm| {
        contains(&ffm.screen, found) || contains(&ffm.screen, b"not installed")
    });

    assert!(shown.is_some(), "the dialog never showed aaa's status");
    assert!(
        contains(&ffm.screen, found),
        "aaa is not found on PATH:\n{}",
        String::from_utf8_lossy(&ffm.screen)
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
    // A failed run must not leave the command behind either. After a passing
    // one the group is already gone, and the kill fails as it should.
    #[expect(
        clippy::let_underscore_must_use,
        reason = "best-effort cleanup: either outcome is fine; drop() of a Copy result is a no-op"
    )]
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

#[expect(
    clippy::expect_used,
    reason = "an assertion helper: failing the test with this message is its job"
)]
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
