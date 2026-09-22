use super::*;
use std::io::{self, Cursor};

struct FakeProcess(Result<Option<Exit>, String>);

impl ManagedProcess for FakeProcess {
    fn poll(&mut self) -> Result<Option<Exit>, String> {
        self.0.clone()
    }
}

fn closed_pipe(bytes: &[u8]) -> Pipe {
    let (sender, receiver) = mpsc::sync_channel(1);
    if !bytes.is_empty() {
        sender.send(Ok(bytes.to_vec())).unwrap();
    }
    drop(sender);
    receiver
}

#[test]
fn capture_preserves_both_streams_and_failure_status() {
    let mut child = FakeProcess(Ok(Some(Exit::Failed(Some(7)))));
    let output = capture(
        &mut child,
        closed_pipe(b"out"),
        closed_pipe(b"err"),
        Duration::from_secs(1),
    )
    .unwrap();
    assert_eq!(output.exit, Exit::Failed(Some(7)));
    assert_eq!(output.stdout.bytes, b"out");
    assert_eq!(output.stderr.bytes, b"err");
}

#[test]
fn verbose_output_is_fully_drained_while_retained_memory_is_bounded() {
    let bytes = vec![b'a'; OUTPUT_LIMIT * 4];
    let pipe = read_pipe(Cursor::new(bytes)).unwrap();
    let mut child = FakeProcess(Ok(Some(Exit::Success)));
    let output = capture(&mut child, pipe, closed_pipe(b""), Duration::from_secs(5)).unwrap();
    assert_eq!(output.stdout.bytes, vec![b'a'; OUTPUT_LIMIT]);
    assert!(output.stdout.truncated);
    assert!(!output.stderr.truncated);
}

#[test]
fn exact_limit_is_not_marked_truncated() {
    let mut output = StreamOutput::default();
    output.append(&vec![b'a'; OUTPUT_LIMIT]);
    assert!(!output.truncated);
    output.append(b"discarded");
    assert!(output.truncated);
    assert_eq!(output.bytes.len(), OUTPUT_LIMIT);
}

#[test]
fn an_unfinished_process_cannot_exceed_the_deadline() {
    let mut child = FakeProcess(Ok(None));
    let error = capture(
        &mut child,
        closed_pipe(b""),
        closed_pipe(b""),
        Duration::ZERO,
    )
    .unwrap_err();
    assert!(error.contains("timed out"));
}

#[test]
fn inherited_open_pipe_cannot_keep_capture_waiting_past_deadline() {
    let (_sender, receiver) = mpsc::sync_channel(1);
    let mut child = FakeProcess(Ok(Some(Exit::Success)));
    let error = capture(&mut child, receiver, closed_pipe(b""), Duration::ZERO).unwrap_err();
    assert!(error.contains("timed out"));
}

#[test]
fn output_progress_after_exit_does_not_extend_the_total_deadline() {
    let (sender, receiver) = mpsc::sync_channel(1);
    sender.send(Ok(b"still writing".to_vec())).unwrap();
    let mut child = FakeProcess(Ok(Some(Exit::Success)));
    let error = capture(&mut child, receiver, closed_pipe(b""), Duration::ZERO).unwrap_err();
    assert!(error.contains("timed out"));
}

#[test]
fn process_poll_failure_is_propagated() {
    let mut child = FakeProcess(Err("process failed".to_owned()));
    assert_eq!(
        capture(
            &mut child,
            closed_pipe(b""),
            closed_pipe(b""),
            Duration::from_secs(1)
        )
        .unwrap_err(),
        "process failed"
    );
}

struct FailedReader;

impl Read for FailedReader {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other("broken pipe fixture"))
    }
}

#[test]
fn pipe_read_failure_is_propagated() {
    let pipe = read_pipe(FailedReader).unwrap();
    let mut child = FakeProcess(Ok(None));
    let error = capture(&mut child, pipe, closed_pipe(b""), Duration::from_secs(1)).unwrap_err();
    assert!(error.contains("broken pipe fixture"));
}

struct InterruptedReader(bool);

impl Read for InterruptedReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if self.0 {
            buffer[0] = b'x';
            Ok(1)
        } else {
            self.0 = true;
            Err(io::ErrorKind::Interrupted.into())
        }
    }
}

#[test]
fn interrupted_reads_are_retried() {
    let pipe = read_pipe(InterruptedReader(false)).unwrap();
    assert_eq!(
        pipe.recv_timeout(Duration::from_secs(1)).unwrap().unwrap(),
        b"x"
    );
    // Dropping the receiver also stops a reader whose bounded queue fills.
}

#[cfg(windows)]
#[test]
fn command_uses_absolute_windows_executable_and_preserves_argument_boundaries() {
    let directory = tempfile::tempdir().unwrap();
    let args = ["--distribution", "Ubuntu; echo text", "--list"];
    let command = command(directory.path(), &args);
    let system_directory = if cfg!(target_pointer_width = "32") {
        "Sysnative"
    } else {
        "System32"
    };
    assert_eq!(
        command.get_program(),
        directory
            .path()
            .join(system_directory)
            .join("wsl.exe")
            .as_os_str()
    );
    assert_eq!(command.get_args().collect::<Vec<_>>(), args);
}
