//! Purpose: Reads every page of a list, and gives up at a page limit rather than listing forever.

use super::call::{self, Call, Transport};
use super::codec;
use crate::values::cloudflare_dns::{CloudflareError, Record, Zone, ZoneId};
use serde_json::Value;

/// The most pages one listing reads: at a hundred records a page, ten thousand records.
const MAX_PAGES: u32 = 100;

pub fn zones(transport: &mut dyn Transport) -> Result<Vec<Zone>, CloudflareError> {
    all_pages(transport, call::zones_page, codec::zones)
}

pub fn records(
    transport: &mut dyn Transport,
    zone: &ZoneId,
) -> Result<Vec<Record>, CloudflareError> {
    all_pages(
        transport,
        |page| call::records_page(zone, page),
        codec::records,
    )
}

/// Every page, in order, from page 1 to the last one the answer names. A failed page fails the
/// whole listing: a partial list would look like a complete one.
fn all_pages<T>(
    transport: &mut dyn Transport,
    request: impl Fn(u32) -> Call,
    decode: fn(&Value) -> Result<Vec<T>, CloudflareError>,
) -> Result<Vec<T>, CloudflareError> {
    let mut items = Vec::new();
    for page in 1..=MAX_PAGES {
        let answer = transport.send(&request(page))?;
        items.extend(decode(&answer)?);
        if page >= codec::total_pages(&answer)? {
            return Ok(items);
        }
    }
    Err(CloudflareError::Limit)
}
