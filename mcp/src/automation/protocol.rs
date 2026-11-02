//! Purpose: Strict response envelopes and bounded JSON-line reads.

use super::Error;
use serde_json::Value;
use tokio::io::{AsyncBufRead, AsyncBufReadExt};

pub(super) const MAX_FRAME_BYTES: usize = 8 * 1024 * 1024;
pub(super) const MAX_REQUEST_BYTES: usize = 4 * 1024 * 1024;

pub(super) async fn read_frame(reader: &mut (impl AsyncBufRead + Unpin)) -> Result<Value, Error> {
    let mut frame = Vec::new();
    loop {
        let available = reader.fill_buf().await?;
        if available.is_empty() {
            return Err(Error::Protocol("FFM closed its automation output"));
        }
        let newline = available.iter().position(|byte| *byte == b'\n');
        let count = newline.map_or(available.len(), |index| index + 1);
        if frame.len().saturating_add(count) > MAX_FRAME_BYTES {
            return Err(Error::Protocol("FFM automation response exceeds 8 MiB"));
        }
        frame.extend_from_slice(
            available
                .get(..count)
                .ok_or(Error::Protocol("invalid frame"))?,
        );
        reader.consume(count);
        if newline.is_some() {
            return serde_json::from_slice(&frame)
                .map_err(|_json| Error::Protocol("FFM returned an invalid JSON response"));
        }
    }
}

pub(super) fn result(envelope: &Value) -> Result<Value, Error> {
    match envelope.get("ok").and_then(Value::as_bool) {
        Some(true) => envelope
            .get("result")
            .cloned()
            .ok_or(Error::Protocol("missing FFM result")),
        Some(false) => application_error(envelope),
        None => Err(Error::Protocol("missing FFM response status")),
    }
}

fn application_error(envelope: &Value) -> Result<Value, Error> {
    let error = envelope
        .get("error")
        .ok_or(Error::Protocol("missing FFM error"))?;
    let code = error
        .get("code")
        .and_then(Value::as_str)
        .ok_or(Error::Protocol("missing error code"))?;
    let message = error
        .get("message")
        .and_then(Value::as_str)
        .ok_or(Error::Protocol("missing error message"))?;
    Err(Error::Application {
        code: code.to_owned(),
        message: message.to_owned(),
    })
}
