//! Purpose: The write requests. A create carries the whole new record, an update only the fields
//! that changed, and an applied change answers with the record as Cloudflare saved it.

use super::call::{self, Transport};
use super::codec;
use crate::values::cloudflare_dns::{
    Change, CloudflareError, NewRecord, Proxy, Record, RecordUpdate, Ttl,
};
use serde_json::{json, Map, Value};

/// Sends one confirmed change. Nothing reaches the transport before the caller holds a `Change`,
/// which the dialog builds only after the user has confirmed it.
pub fn apply(transport: &mut dyn Transport, change: &Change) -> Result<Record, CloudflareError> {
    let answer = match change {
        Change::Create(new) => transport.send(&call::create(&new.zone.id, create_body(new)))?,
        Change::Update(update) => transport.send(&call::update(
            &update.zone.id,
            &update.record.id,
            patch_body(update),
        ))?,
    };
    codec::record(&answer)
}

pub fn create_body(new: &NewRecord) -> Value {
    json!({
        "type": "A",
        "name": new.name,
        "content": new.content,
        "ttl": ttl_wire(new.ttl),
        "proxied": new.proxy == Proxy::On,
    })
}

/// The fields that differ from the record. The type and the name are never sent.
pub fn patch_body(update: &RecordUpdate) -> Value {
    let changed = update.changed();
    let mut body = Map::new();
    if let Some(content) = changed.content {
        body.insert("content".to_owned(), Value::String(content));
    }
    if let Some(ttl) = changed.ttl {
        body.insert("ttl".to_owned(), json!(ttl_wire(ttl)));
    }
    if let Some(proxy) = changed.proxy {
        body.insert("proxied".to_owned(), json!(proxy == Proxy::On));
    }
    Value::Object(body)
}

/// Cloudflare spells automatic as `1`.
const fn ttl_wire(ttl: Ttl) -> u32 {
    match ttl {
        Ttl::Automatic => 1,
        Ttl::Seconds(seconds) => seconds,
    }
}
