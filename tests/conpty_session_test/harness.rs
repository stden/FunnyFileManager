//! Purpose: Windows `ConPTY` pseudo-console harness for driving the real `ffm` binary.

use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use tempfile::TempDir;

pub const STARTUP: Duration = Duration::from_secs(30);
pub const PATIENCE: Duration = Duration::from_secs(20);

pub struct ConPtyFfm {
    _master: Box<dyn MasterPty + Send>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    child: Box<dyn Child + Send + Sync>,
    screen: Arc<Mutex<Vec<u8>>>,
    sandbox: TempDir,
}

impl ConPtyFfm {
    #[expect(clippy::expect_used, reason = "test fixture setup")]
    pub fn start(rows: u16, args: &[&str]) -> Self {
        let sandbox = TempDir::new().expect("sandbox");
        for sub in ["work", "home", "appdata", "tmp"] {
            fs::create_dir_all(sandbox.path().join(sub)).expect("mkdir");
        }
        fs::write(
            sandbox.path().join("work/demo.cmd"),
            "@echo off\r\necho program-completed\r\nexit /b 7\r\n",
        )
        .expect("demo.cmd");
        open_conpty_session(sandbox, rows, args)
    }

    pub fn work(&self) -> PathBuf {
        self.sandbox.path().join("work")
    }

    #[expect(clippy::expect_used, reason = "test fixture screen mutex")]
    pub fn clear_screen(&self) {
        self.screen.lock().expect("screen lock").clear();
    }

    #[expect(clippy::expect_used, reason = "test fixture writer")]
    pub fn send(&self, bytes: &[u8]) {
        let mut writer = self.writer.lock().expect("writer lock");
        writer.write_all(bytes).expect("write keys");
        writer.flush().expect("flush keys");
    }

    #[expect(clippy::expect_used, reason = "test fixture screen mutex")]
    pub fn wait_for(&self, limit: Duration, mut pred: impl FnMut(&[u8]) -> bool) -> bool {
        let start = Instant::now();
        while start.elapsed() < limit {
            if pred(&self.screen.lock().expect("screen lock")) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(15));
        }
        false
    }

    #[expect(clippy::expect_used, reason = "test fixture child status")]
    pub fn quit(&mut self, limit: Duration) -> bool {
        let start = Instant::now();
        let mut next_send = Instant::now();
        while start.elapsed() < limit {
            if Instant::now() >= next_send {
                self.send(b"\x03");
                next_send = Instant::now() + Duration::from_millis(400);
            }
            if self.child.try_wait().expect("try_wait").is_some() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(15));
        }
        false
    }
}

impl Drop for ConPtyFfm {
    fn drop(&mut self) {
        drop(self.child.kill());
        drop(self.child.wait());
    }
}

fn append_and_count_queries(sink: &Mutex<Vec<u8>>, chunk: &[u8]) -> usize {
    let Ok(mut out) = sink.lock() else {
        return 0;
    };
    let tail = out.len().saturating_sub(3);
    out.extend_from_slice(chunk);
    out.get(tail..).map_or(0, |slice| {
        slice.windows(4).filter(|w| *w == b"\x1b[6n").count()
    })
}

fn reply_cursor_queries(responder: &Mutex<Box<dyn Write + Send>>, count: usize) {
    let Ok(mut writer) = responder.lock() else {
        return;
    };
    for _ in 0..count {
        drop(writer.write_all(b"\x1b[1;1R"));
    }
    drop(writer.flush());
}

fn spawn_reader(
    mut reader: Box<dyn Read + Send>,
    sink: Arc<Mutex<Vec<u8>>>,
    responder: Arc<Mutex<Box<dyn Write + Send>>>,
) {
    std::thread::spawn(move || {
        let mut buf = [0_u8; 4096];
        while let Ok(n @ 1..) = reader.read(&mut buf) {
            let queries = buf
                .get(..n)
                .map_or(0, |chunk| append_and_count_queries(&sink, chunk));
            if queries > 0 {
                reply_cursor_queries(&responder, queries);
            }
        }
    });
}

#[expect(clippy::expect_used, reason = "test fixture ConPTY open and spawn")]
fn open_conpty_session(sandbox: TempDir, rows: u16, args: &[&str]) -> ConPtyFfm {
    let size = PtySize {
        rows,
        cols: 100,
        pixel_width: 0,
        pixel_height: 0,
    };
    let pair = native_pty_system().openpty(size).expect("openpty");
    let reader = pair.master.try_clone_reader().expect("clone reader");
    let writer = Arc::new(Mutex::new(pair.master.take_writer().expect("take writer")));
    let child = pair
        .slave
        .spawn_command(build_ffm_command(&sandbox, args))
        .expect("spawn ffm");
    drop(pair.slave);
    let screen = Arc::new(Mutex::new(Vec::new()));
    spawn_reader(reader, Arc::clone(&screen), Arc::clone(&writer));
    let session = ConPtyFfm {
        _master: pair.master,
        writer,
        child,
        screen,
        sandbox,
    };
    assert!(session.wait_for(STARTUP, |s| contains(s, b"Quit")));
    session
}

fn build_ffm_command(sandbox: &TempDir, args: &[&str]) -> CommandBuilder {
    let mut cmd = CommandBuilder::new(env!("CARGO_BIN_EXE_ffm"));
    cmd.args(args);
    cmd.cwd(sandbox.path().join("work"));
    for (k, sub) in [
        ("HOME", "home"),
        ("USERPROFILE", "home"),
        ("APPDATA", "appdata"),
        ("LOCALAPPDATA", "appdata"),
        ("XDG_CONFIG_HOME", "appdata"),
        ("XDG_DATA_HOME", "appdata"),
        ("TEMP", "tmp"),
        ("TMP", "tmp"),
    ] {
        cmd.env(k, sandbox.path().join(sub));
    }
    if let Some(prof) = std::env::var_os("LLVM_PROFILE_FILE") {
        cmd.env("LLVM_PROFILE_FILE", prof);
    }
    cmd
}

pub fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}
