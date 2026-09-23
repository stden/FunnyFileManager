//! Purpose: Bounded byte-framed LSP transport, independent of processes and terminals.

use serde_json::Value;
use std::io::{self, Read, Write};

const MAX_HEADER: usize = 8 * 1024;
const MAX_BODY: usize = 32 * 1024 * 1024;

pub(super) fn read_frame(input: &mut impl Read) -> io::Result<Option<Value>> {
    let mut header = Vec::new();
    let mut byte = [0];
    while !header.ends_with(b"\r\n\r\n") {
        match input.read_exact(&mut byte) {
            Ok(()) => header.push(byte[0]),
            Err(error) if error.kind() == io::ErrorKind::UnexpectedEof && header.is_empty() => {
                return Ok(None)
            }
            Err(error) => return Err(error),
        }
        if header.len() > MAX_HEADER {
            return Err(invalid("LSP header too large"));
        }
    }
    let length = content_length(&header)?;
    let mut body = vec![0; length];
    input.read_exact(&mut body)?;
    serde_json::from_slice(&body)
        .map(Some)
        .map_err(|_| invalid("Invalid LSP JSON"))
}

fn content_length(header: &[u8]) -> io::Result<usize> {
    let text = std::str::from_utf8(header).map_err(|_| invalid("Invalid LSP header"))?;
    let mut length = None;
    for line in text.split("\r\n").filter(|line| !line.is_empty()) {
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| invalid("Invalid LSP header"))?;
        if name.eq_ignore_ascii_case("Content-Length") {
            if length.is_some() {
                return Err(invalid("Duplicate LSP length"));
            }
            let value = value
                .trim()
                .parse::<usize>()
                .map_err(|_| invalid("Invalid LSP length"))?;
            if value > MAX_BODY {
                return Err(invalid("LSP message too large"));
            }
            length = Some(value);
        }
    }
    length.ok_or_else(|| invalid("Missing LSP length"))
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

pub(super) fn write_frame(output: &mut impl Write, value: &Value) -> io::Result<()> {
    let body = serde_json::to_vec(value)?;
    if body.len() > MAX_BODY {
        return Err(invalid("LSP message too large"));
    }
    write!(output, "Content-Length: {}\r\n\r\n", body.len())?;
    output.write_all(&body)?;
    output.flush()
}
