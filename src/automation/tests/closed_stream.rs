//! Purpose: A response stream that fails on flush ends the session with that failure.
use crate::automation::{options::Options, server};
use std::io;

/// Accepts every byte but cannot deliver them: the reader has gone.
struct ClosedOnFlush;

impl io::Write for ClosedOnFlush {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(io::ErrorKind::BrokenPipe.into())
    }
}

#[test]
fn a_closed_response_stream_stops_the_session() {
    let root = tempfile::TempDir::new().unwrap();
    let options =
        Options::parse(&["--root".into(), root.path().into(), "--isolated".into()]).unwrap();
    let error = server::serve(options, io::Cursor::new(b""), &mut ClosedOnFlush)
        .expect_err("closed stream");
    assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
}
