//! Purpose: JSON framing bounds memory and preserves recoverable transport errors.
use super::*;
use std::io::Cursor;

#[test]
fn oversized_frames_stop_at_the_limit_and_eof_is_not_a_request() {
    let mut input = Cursor::new(vec![b' '; MAX_FRAME + 2]);
    assert_eq!(
        read(&mut input).err().unwrap().kind(),
        io::ErrorKind::FileTooLarge
    );
    assert_eq!(input.position(), u64::try_from(MAX_FRAME).unwrap() + 1);
    assert!(read(&mut Cursor::new([])).unwrap().is_none());
    assert_eq!(
        read(&mut Cursor::new(b"invalid\n")).err().unwrap().kind(),
        io::ErrorKind::InvalidInput
    );
}

#[test]
fn error_kinds_have_stable_machine_readable_codes() {
    let cases = [
        (io::ErrorKind::InvalidInput, "invalid_input"),
        (io::ErrorKind::FileTooLarge, "frame_too_large"),
        (io::ErrorKind::WouldBlock, "busy"),
        (io::ErrorKind::BrokenPipe, "session_closed"),
        (io::ErrorKind::PermissionDenied, "permission_denied"),
        (io::ErrorKind::Other, "operation_failed"),
    ];
    for (kind, expected) in cases {
        let mut output = Vec::new();
        write(&mut output, Err(io::Error::new(kind, "fixture failure"))).unwrap();
        assert_eq!(output.last(), Some(&b'\n'));
        let response: Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(response["ok"], false);
        assert_eq!(response["error"]["code"], expected);
        assert_eq!(response["error"]["message"], "fixture failure");
    }
}

struct FailedWriter;
impl Write for FailedWriter {
    fn write(&mut self, _bytes: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed output"))
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn transport_does_not_hide_output_failures() {
    assert!(write(&mut FailedWriter, Ok(json!({"state": "ready"}))).is_err());
}

#[test]
fn omitted_wait_timeout_uses_the_protocol_default() {
    let request = read(&mut Cursor::new(b"{\"operation\":\"wait\"}\n"))
        .unwrap()
        .unwrap();
    assert!(matches!(request, Request::Wait { timeout_ms: 1000 }));
}
