//! Purpose: Exercise Unix reader retry, EOF and error semantics on a private PTY.

use super::super::*;
use std::collections::VecDeque;

struct ScriptedRead(VecDeque<io::Result<Vec<u8>>>);

impl Read for ScriptedRead {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let bytes = self.0.pop_front().unwrap_or(Ok(Vec::new()))?;
        let destination = buffer
            .get_mut(..bytes.len())
            .ok_or_else(|| io::Error::other("fixture too large"))?;
        destination.copy_from_slice(&bytes);
        Ok(bytes.len())
    }
}

struct Fixture {
    capture: Capture,
    ready: readiness::Capture,
    captured: Arc<Mutex<Captured>>,
    host: HeadlessHost,
}

fn fixture() -> io::Result<Fixture> {
    let host = HeadlessHost::new();
    let (sender, _input) = mpsc::sync_channel(1);
    let generation = host.begin(sender)?;
    let (replies, _responses) = mpsc::sync_channel(1);
    let captured = Arc::new(Mutex::new(Captured::default()));
    let pair = portable_pty::native_pty_system()
        .openpty(host.pty_size())
        .map_err(io::Error::other)?;
    let master = Master::new(pair.master);
    let ready = readiness::Reader::new(readiness::Lease::new(&master))?;
    drop(pair.slave); // Its closed peer makes only this private master readable.
    Ok(Fixture {
        capture: Capture::new(host.clone(), generation, Arc::clone(&captured), replies),
        ready,
        captured,
        host,
    })
}

#[test]
fn transient_read_errors_retry_without_losing_later_output() {
    let mut fixture = fixture().unwrap();
    let mut reader = ScriptedRead(VecDeque::from([
        Err(io::Error::from(io::ErrorKind::Interrupted)),
        Err(io::Error::from(io::ErrorKind::WouldBlock)),
        Ok(b"private-output".to_vec()),
        Ok(Vec::new()),
    ]));
    read_all(
        &mut reader,
        &mut fixture.capture,
        fixture.ready,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(fixture.captured.lock().unwrap().bytes(), b"private-output");
    assert_eq!(fixture.host.snapshot().output, "private-output");
}

#[test]
fn non_pty_read_failure_is_reported_and_pty_eio_is_normal_eof() {
    for (error, expected) in [
        (
            io::Error::from(io::ErrorKind::PermissionDenied),
            Some(io::ErrorKind::PermissionDenied),
        ),
        (io::Error::from_raw_os_error(5), None),
    ] {
        let mut fixture = fixture().unwrap();
        let mut reader = ScriptedRead(VecDeque::from([Err(error)]));
        let result = read_all(
            &mut reader,
            &mut fixture.capture,
            fixture.ready,
            &AtomicBool::new(false),
        );
        assert_eq!(result.err().map(|error| error.kind()), expected);
        assert_eq!(fixture.host.snapshot().output, "");
    }
}
