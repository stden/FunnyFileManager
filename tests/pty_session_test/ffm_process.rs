//! Purpose: The real ffm binary on its own pseudo-terminal, and opening that terminal.

use super::*;

/// The first frame of a debug build, on a machine busy with the rest of the suite.
const STARTUP: Duration = Duration::from_secs(60);
/// How often the test looks at the screen, the files and the process.
const TICK: Duration = Duration::from_millis(5);

/// `ffm` on its own pseudo-terminal, in a sandbox home and working directory.
pub struct Ffm {
    /// The terminal's far end. Dropping it hangs the terminal up.
    terminal: Option<File>,
    pub process: Child,
    pub screen: Vec<u8>,
    pub sandbox: TempDir,
}

impl Ffm {
    #[expect(
        clippy::expect_used,
        reason = "test fixture: an error from the pty or the ffm process fails the test driving it"
    )]
    pub fn start() -> Self {
        let path = std::env::var_os("PATH").unwrap_or_default();
        Self::start_in(TempDir::new().expect("sandbox"), &path, &[])
    }

    /// `ffm args` with `sandbox/home` as its home, `sandbox/tmp` as its
    /// temporary directory, `sandbox/work` as its working directory and `path`
    /// as its `PATH`; whatever the sandbox holds already is there when it
    /// starts.
    #[expect(
        clippy::expect_used,
        reason = "test fixture: an error from the pty or the ffm process fails the test driving it"
    )]
    pub fn start_in(sandbox: TempDir, path: &OsStr, args: &[&str]) -> Self {
        let home = sandbox.path().join("home");
        let tmp = sandbox.path().join("tmp");
        for directory in [&home, &tmp, &sandbox.path().join("work")] {
            fs::create_dir_all(directory).expect("sandbox directory");
        }
        let (terminal, device) = open_pty();
        let process = Command::new(env!("CARGO_BIN_EXE_ffm"))
            .args(args)
            .current_dir(sandbox.path().join("work"))
            .env_clear()
            .env("PATH", path)
            .env("HOME", &home)
            .env("TMPDIR", &tmp)
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

    pub fn work(&self) -> PathBuf {
        self.sandbox.path().join("work")
    }

    /// Send `keys` in one write, the way a paste or a fast typist's burst lands.
    #[expect(
        clippy::expect_used,
        reason = "test fixture: an error from the pty or the ffm process fails the test driving it"
    )]
    pub fn type_keys(&mut self, keys: &[u8]) {
        let terminal = self.terminal.as_mut().expect("the terminal is up");
        terminal.write_all(keys).expect("type");
    }

    pub fn hang_up(&mut self) {
        self.terminal = None;
    }

    /// Send ffm `signal`, the way `tmux kill-session` sends the pane SIGHUP.
    #[expect(
        clippy::expect_used,
        reason = "test fixture: an error from the pty or the ffm process fails the test driving it"
    )]
    pub fn signal(&self, signal: Signal) {
        kill_process(Pid::from_child(&self.process), signal).expect("signal ffm");
    }

    /// How ffm ended, or `None` if it is still running after [`HANG_UP`].
    #[expect(
        clippy::expect_used,
        reason = "test fixture: an error from the pty or the ffm process fails the test driving it"
    )]
    pub fn wait_for_exit(&mut self) -> Option<ExitStatus> {
        self.wait_for(HANG_UP, |ffm| {
            ffm.process.try_wait().expect("ffm status").is_some()
        })?;
        self.process.try_wait().expect("ffm status")
    }

    /// Poll until `done`, reading the screen so ffm never blocks on a full
    /// terminal. The time it took, or `None` past `limit`.
    pub fn wait_for(
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
