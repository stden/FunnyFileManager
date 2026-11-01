//! Purpose: Exercise pipe failure reporting and stderr draining with deterministic I/O adapters.

use super::*;
use std::io::{Read, Write};

struct Broken;
impl Write for Broken {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::ErrorKind::BrokenPipe.into())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn a_broken_pipe_reports_failure_instead_of_waiting_for_another_message() {
    let (sender, receiver) = mpsc::channel();
    sender
        .send(serde_json::json!({"jsonrpc":"2.0","method":"exit"}))
        .unwrap();
    let (events, incoming) = mpsc::sync_channel(1);
    writer(Broken, &receiver, &events);
    assert!(matches!(incoming.try_recv().unwrap(), Signal::Failed));
}

struct Stderr {
    reads: usize,
}
impl Read for Stderr {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.reads += 1;
        match self.reads {
            1 => Err(io::ErrorKind::Interrupted.into()),
            2 => {
                bytes[0] = b'x';
                Ok(1)
            }
            _ => Err(io::ErrorKind::BrokenPipe.into()),
        }
    }
}

#[test]
fn stderr_is_drained_through_interruptions_and_stops_after_failure() {
    let mut stderr = Stderr { reads: 0 };
    assert_eq!(drain_stderr(&mut stderr), None);
    assert_eq!(stderr.reads, 3);
}

#[test]
fn toolchain_hint_is_consumed_once_and_handles_a_panicked_stderr_task() {
    let root = tempfile::TempDir::new().unwrap();
    let settings = ServerSettings {
        command: "python".into(),
        args: vec!["-c".into(), "pass".into()],
        ..ServerSettings::default()
    };
    let mut process = Process::start(&settings, root.path()).unwrap();
    process.child.wait().unwrap();
    assert_eq!(process.stderr_task.take().unwrap().join().unwrap(), None);
    process.stderr_task = Some(thread::spawn(|| Some("missing component")));
    assert_eq!(process.take_toolchain_hint(), Some("missing component"));
    assert_eq!(process.take_toolchain_hint(), None);
    assert!(process.terminated);
    process.stderr_task = Some(thread::spawn(|| panic!("injected stderr failure")));
    assert_eq!(process.take_toolchain_hint(), None);
}

#[test]
fn rustup_proxy_stderr_yields_actionable_hint_while_random_stderr_does_not() {
    let rust_analyzer =
        b"error: 'rust-analyzer.exe' is not installed for the toolchain '1.99.1-x86_64-pc-windows-msvc'\r\n";
    assert_eq!(
        drain_stderr(&mut &rust_analyzer[..]),
        Some("LSP unavailable: run 'rustup component add rust-analyzer' (Ctrl+Alt+R retries)")
    );
    let unknown_bin =
        b"error: Unknown binary 'clippy-driver' in official toolchain 'stable-x86_64-unknown-linux-gnu'.\n";
    assert_eq!(
        drain_stderr(&mut &unknown_bin[..]),
        Some("LSP unavailable: missing rustup toolchain component (Ctrl+Alt+R retries)")
    );
    assert_eq!(drain_stderr(&mut &b"ordinary crash output\n"[..]), None);
}

#[test]
fn disconnected_pipes_preserve_unsent_payloads_and_cleanup_survives_task_panics() {
    let root = tempfile::TempDir::new().unwrap();
    let settings = ServerSettings {
        command: "python".into(),
        args: vec!["-c".into(), "pass".into()],
        ..ServerSettings::default()
    };
    let mut process = Process::start(&settings, root.path()).unwrap();
    process.outgoing.take();
    let value = serde_json::json!({"method":"shutdown"});
    assert!(
        matches!(process.send(value.clone()), Err(mpsc::TrySendError::Disconnected(returned)) if returned == value)
    );
    let (sender, receiver) = mpsc::sync_channel(1);
    process.incoming = Some(receiver);
    drop(sender);
    assert!(matches!(process.poll(), Some(Signal::Closed)));
    process
        .threads
        .push(thread::spawn(|| panic!("injected task failure")));
    drop(process); // A failed I/O task must not abort shutdown or leak the child.
}

#[cfg(unix)]
#[test]
fn observing_exit_keeps_the_process_id_reserved_until_group_cleanup() {
    use rustix::process::{waitid, Pid, WaitId, WaitIdOptions};
    let root = tempfile::TempDir::new().unwrap();
    let settings = ServerSettings {
        command: "python".into(),
        args: vec!["-c".into(), "pass".into()],
        ..ServerSettings::default()
    };
    let mut process = Process::start(&settings, root.path()).unwrap();
    let pid = Pid::from_child(&process.child);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !process.exited() {
        assert!(std::time::Instant::now() < deadline);
        thread::sleep(std::time::Duration::from_millis(5));
    }
    let flags = WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT;
    assert!(waitid(WaitId::Pid(pid), flags).unwrap().is_some());
    drop(process);
    assert_eq!(
        waitid(WaitId::Pid(pid), flags).unwrap_err(),
        rustix::io::Errno::CHILD
    );
}
