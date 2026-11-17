//! Purpose: The requests the client sends: method, path with every id percent-encoded, query and
//! body. The methods are the three the editor uses; there is no DELETE.

use crate::values::cloudflare_dns::{CloudflareError, RecordId, ZoneId};
use serde_json::Value;

/// Zones read per page: the most the zone listing accepts.
const ZONES_PER_PAGE: u32 = 50;
/// Records read per page.
const RECORDS_PER_PAGE: u32 = 100;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Patch,
}

/// One request, relative to the API base.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Call {
    pub(crate) method: Method,
    pub(crate) path: String,
    pub(crate) query: Vec<(&'static str, String)>,
    pub(crate) body: Option<Value>,
}

/// Sends one call and returns the decoded JSON answer.
pub trait Transport {
    fn send(&mut self, call: &Call) -> Result<Value, CloudflareError>;
}

/// One path segment, percent-encoded: every byte outside the unreserved set becomes `%XX`, so an
/// id can never add a `/`, a `?` or a `#` to the path.
pub fn segment(text: &str) -> String {
    let mut encoded = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push('%');
            push_hex_digit(&mut encoded, byte >> 4);
            push_hex_digit(&mut encoded, byte & 0x0f);
        }
    }
    encoded
}

fn push_hex_digit(text: &mut String, nibble: u8) {
    let digit = char::from_digit(u32::from(nibble), 16).unwrap_or('0');
    text.push(digit.to_ascii_uppercase());
}

pub fn zones_page(page: u32) -> Call {
    Call {
        method: Method::Get,
        path: "/zones".to_owned(),
        query: paged(page, ZONES_PER_PAGE),
        body: None,
    }
}

pub fn records_page(zone: &ZoneId, page: u32) -> Call {
    Call {
        method: Method::Get,
        path: records_path(zone),
        query: paged(page, RECORDS_PER_PAGE),
        body: None,
    }
}

pub fn create(zone: &ZoneId, body: Value) -> Call {
    Call {
        method: Method::Post,
        path: records_path(zone),
        query: Vec::new(),
        body: Some(body),
    }
}

pub fn update(zone: &ZoneId, record: &RecordId, body: Value) -> Call {
    Call {
        method: Method::Patch,
        path: format!("{}/{}", records_path(zone), segment(record.as_str())),
        query: Vec::new(),
        body: Some(body),
    }
}

fn records_path(zone: &ZoneId) -> String {
    format!("/zones/{}/dns_records", segment(zone.as_str()))
}

fn paged(page: u32, per_page: u32) -> Vec<(&'static str, String)> {
    vec![
        ("page", page.to_string()),
        ("per_page", per_page.to_string()),
    ]
}
