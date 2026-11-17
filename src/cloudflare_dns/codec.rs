//! Purpose: Cloudflare answers decoded into zones, records and page counts, and error envelopes
//! reduced to their first error, with the text made safe to show on one line.

use crate::values::cloudflare_dns::{CloudflareError, Proxy, Record, RecordId, Ttl, Zone, ZoneId};
use serde_json::Value;

/// The longest error message shown.
const MESSAGE_CHARS: usize = 200;
/// The TTL Cloudflare reports for automatic.
const AUTOMATIC_TTL: u64 = 1;

pub fn zones(value: &Value) -> Result<Vec<Zone>, CloudflareError> {
    items(value)?.iter().map(zone).collect()
}

pub fn records(value: &Value) -> Result<Vec<Record>, CloudflareError> {
    items(value)?.iter().map(record_value).collect()
}

/// The record in a create or update answer.
pub fn record(value: &Value) -> Result<Record, CloudflareError> {
    record_value(
        value
            .get("result")
            .ok_or(CloudflareError::InvalidResponse)?,
    )
}

pub fn total_pages(value: &Value) -> Result<u32, CloudflareError> {
    value
        .get("result_info")
        .and_then(|info| info.get("total_pages"))
        .and_then(Value::as_u64)
        .and_then(|pages| u32::try_from(pages).ok())
        .ok_or(CloudflareError::InvalidResponse)
}

/// Cloudflare's refusal: `success` false, with an `errors` list. Only its first error is kept.
pub fn error_envelope(value: &Value) -> Option<CloudflareError> {
    if value.get("success").and_then(Value::as_bool) != Some(false) {
        return None;
    }
    let first = value
        .get("errors")
        .and_then(Value::as_array)
        .and_then(|errors| errors.first());
    Some(match first {
        Some(error) => CloudflareError::Api {
            code: error.get("code").and_then(Value::as_u64).unwrap_or(0),
            message: error
                .get("message")
                .and_then(Value::as_str)
                .map_or_else(|| "no message given".to_owned(), sanitised),
        },
        None => CloudflareError::InvalidResponse,
    })
}

fn items(value: &Value) -> Result<&Vec<Value>, CloudflareError> {
    value
        .get("result")
        .and_then(Value::as_array)
        .ok_or(CloudflareError::InvalidResponse)
}

fn zone(value: &Value) -> Result<Zone, CloudflareError> {
    Ok(Zone {
        id: ZoneId::new(text(value, "id")?),
        name: text(value, "name")?.to_owned(),
    })
}

fn record_value(value: &Value) -> Result<Record, CloudflareError> {
    Ok(Record {
        id: RecordId::new(text(value, "id")?),
        kind: text(value, "type")?.to_owned(),
        name: text(value, "name")?.to_owned(),
        content: text(value, "content")?.to_owned(),
        ttl: ttl(value)?,
        proxy: proxy(value)?,
    })
}

fn text<'a>(value: &'a Value, field: &str) -> Result<&'a str, CloudflareError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or(CloudflareError::InvalidResponse)
}

fn ttl(value: &Value) -> Result<Ttl, CloudflareError> {
    let seconds = value
        .get("ttl")
        .and_then(Value::as_u64)
        .ok_or(CloudflareError::InvalidResponse)?;
    if seconds == AUTOMATIC_TTL {
        return Ok(Ttl::Automatic);
    }
    u32::try_from(seconds)
        .map(Ttl::Seconds)
        .map_err(|_error| CloudflareError::InvalidResponse)
}

/// `proxied` is absent for the record types that cannot be proxied.
fn proxy(value: &Value) -> Result<Proxy, CloudflareError> {
    match value.get("proxied") {
        None => Ok(Proxy::Unsupported),
        Some(Value::Bool(true)) => Ok(Proxy::On),
        Some(Value::Bool(false)) => Ok(Proxy::Off),
        Some(_) => Err(CloudflareError::InvalidResponse),
    }
}

/// The message without control characters, which could move the cursor or change the colours of
/// the screen, and capped in length.
fn sanitised(text: &str) -> String {
    text.chars()
        .filter(|ch| !ch.is_control())
        .take(MESSAGE_CHARS)
        .collect()
}
