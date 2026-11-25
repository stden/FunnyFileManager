//! Purpose: Automation framing is bounded and every output line is one complete JSON value.
use super::{
    options::invalid,
    protocol::{Request, MAX_FRAME},
};
use serde_json::{json, Value};
use std::io::{self, BufRead, Read, Write};

pub(super) fn read(reader: &mut impl BufRead) -> io::Result<Option<Request>> {
    let mut frame = Vec::new();
    let limit = u64::try_from(MAX_FRAME).map_err(io::Error::other)? + 1;
    let size = reader.take(limit).read_until(b'\n', &mut frame)?;
    if size == 0 {
        return Ok(None);
    }
    if size > MAX_FRAME {
        return Err(io::Error::new(
            io::ErrorKind::FileTooLarge,
            "automation frame is too large",
        ));
    }
    serde_json::from_slice(&frame)
        .map(Some)
        .map_err(|_error| invalid("invalid automation request"))
}

pub(super) fn write(writer: &mut impl Write, result: io::Result<Value>) -> io::Result<()> {
    let response = match result {
        Ok(result) => json!({"ok": true, "result": result}),
        Err(error) => failure(&error),
    };
    serde_json::to_writer(&mut *writer, &response).map_err(io::Error::other)?;
    writer.write_all(b"\n")?;
    writer.flush()
}

fn failure(error: &io::Error) -> Value {
    let code = match error.kind() {
        io::ErrorKind::InvalidInput => "invalid_input",
        io::ErrorKind::FileTooLarge => "frame_too_large",
        io::ErrorKind::WouldBlock => "busy",
        io::ErrorKind::BrokenPipe => "session_closed",
        io::ErrorKind::PermissionDenied => "permission_denied",
        _ => "operation_failed",
    };
    json!({"ok": false, "error": {"code": code, "message": error.to_string()}})
}

#[cfg(test)]
mod tests;
