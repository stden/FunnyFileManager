//! Unit tests: the real native clipboard on a private X server, opt-in through
//! `FFM_TEST_CLIPBOARD_SERVER=1` because it needs `Xvfb`.

use super::*;
use std::{
    io::{BufRead, BufReader},
    path::Path,
    process::{Child, ChildStdout, Command, Stdio},
    sync::mpsc,
    time::Duration,
};

struct DisplayServer(Child);

impl DisplayServer {
    fn start() -> (Self, String) {
        let child = Command::new("Xvfb")
            .args([
                "-displayfd",
                "1",
                "-screen",
                "0",
                "640x480x24",
                "-nolisten",
                "tcp",
                "-ac",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("Xvfb is required when FFM_TEST_CLIPBOARD_SERVER=1");
        let mut server = Self(child);
        let stdout = server.0.stdout.take().expect("display readiness pipe");
        let display = read_display(stdout);
        (server, display)
    }
}

impl Drop for DisplayServer {
    fn drop(&mut self) {
        drop(self.0.kill());
        drop(self.0.wait());
    }
}

fn read_display(stdout: ChildStdout) -> String {
    let (sender, receiver) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut number = String::new();
        let result = BufReader::new(stdout)
            .read_line(&mut number)
            .map(|_| number);
        drop(sender.send(result));
    });
    let number = receiver
        .recv_timeout(Duration::from_secs(10))
        .expect("Xvfb must announce its private display")
        .expect("Xvfb readiness is readable");
    reader.join().expect("display readiness reader");
    let number: u16 = number.trim().parse().expect("Xvfb display number");
    format!(":{number}")
}

#[test]
fn native_text_round_trips_on_a_private_display() {
    if std::env::var("FFM_TEST_CLIPBOARD_SERVER").as_deref() != Ok("1") {
        return;
    }
    let (_server, display) = DisplayServer::start();
    let _display = crate::test_support::EnvVarGuard::set("DISPLAY", Path::new(&display));
    let mut clipboard = SystemClipboard::new(Box::new(Vec::new()));
    clipboard
        .set_native("fixture clipboard text")
        .expect("native write");
    assert_eq!(
        clipboard.get_native().expect("native read"),
        "fixture clipboard text"
    );
    clipboard.set_native("").expect("clear fixture clipboard");
}
