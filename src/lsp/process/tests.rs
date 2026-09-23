//! Purpose: Exercise pipe failure reporting and stderr draining with deterministic I/O adapters.

use super::*;
use std::io::Write;

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
    drain_stderr(&mut stderr);
    assert_eq!(stderr.reads, 3);
}
