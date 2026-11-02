//! Purpose: Framing regression tests, including fragmented and malformed byte streams.

use super::super::transport::{read_frame, write_frame};
use serde_json::json;
use std::io::{self, Cursor, Read};

struct Fragmented(Cursor<Vec<u8>>);

impl Read for Fragmented {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.0.read(&mut buffer[..1])
    }
}

#[test]
fn frames_use_utf8_byte_lengths_and_survive_single_byte_reads() {
    let value = json!({"jsonrpc":"2.0", "method":"example", "params":"Привет 😀"});
    let mut bytes = Vec::new();
    write_frame(&mut bytes, &value).unwrap();
    let header = format!(
        "Content-Length: {}\r\n\r\n",
        serde_json::to_vec(&value).unwrap().len()
    );
    assert!(bytes.starts_with(header.as_bytes()));
    assert_eq!(
        read_frame(&mut Fragmented(Cursor::new(bytes))).unwrap(),
        Some(value)
    );
}

#[test]
fn consecutive_frames_remain_separate_and_eof_is_clean() {
    let mut bytes = Vec::new();
    write_frame(&mut bytes, &json!({"id":1})).unwrap();
    write_frame(&mut bytes, &json!({"id":2})).unwrap();
    let mut reader = Cursor::new(bytes);
    assert_eq!(read_frame(&mut reader).unwrap(), Some(json!({"id":1})));
    assert_eq!(read_frame(&mut reader).unwrap(), Some(json!({"id":2})));
    assert_eq!(read_frame(&mut reader).unwrap(), None);
}

#[test]
fn content_type_can_precede_case_insensitive_length() {
    let bytes =
        b"Content-Type: application/vscode-jsonrpc; charset=utf-8\r\ncontent-length: 2\r\n\r\n{}";
    assert_eq!(read_frame(&mut &bytes[..]).unwrap(), Some(json!({})));
}

#[test]
fn malformed_or_truncated_frames_are_errors() {
    for bytes in [
        b"Content-Length: x\r\n\r\n{}".as_slice(),
        b"Content-Length: -1\r\n\r\n",
        b"Content-Length: 999999999\r\n\r\n",
        b"Content-Length: 2\r\nContent-Length: 2\r\n\r\n{}",
        b"Content-Type: application/json\r\n\r\n{}",
        b"Content-Length: 2\r\n\r\nxx",
        b"Content-Length: 8\r\n\r\n{}",
        b"Content-Length: 2\r",
    ] {
        assert!(read_frame(&mut &*bytes).is_err(), "accepted {bytes:?}");
    }
}
#[test]
fn oversized_and_non_utf8_headers_are_rejected_without_reading_a_body() {
    for frame in [
        vec![b'x'; 8193],
        b"\xff: value\r\nContent-Length: 2\r\n\r\n{}".to_vec(),
        b"broken header\r\n\r\n".to_vec(),
    ] {
        assert!(read_frame(&mut frame.as_slice()).is_err());
    }
}

/// The three parse failures keep their own fixed message and kind: the parser's
/// detail goes to the diagnostics log, never into the returned error.
#[test]
fn a_malformed_frame_names_the_part_that_is_invalid() {
    for (frame, message) in [
        (
            b"Content-Length: 2\r\n\r\nxx".as_slice(),
            "Invalid LSP JSON",
        ),
        (
            b"\xff: value\r\nContent-Length: 2\r\n\r\n{}".as_slice(),
            "Invalid LSP header",
        ),
        (
            b"Content-Length: x\r\n\r\n{}".as_slice(),
            "Invalid LSP length",
        ),
    ] {
        let error = read_frame(&mut &*frame).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData, "{message}");
        assert_eq!(error.to_string(), message);
    }
}
