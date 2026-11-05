//! Purpose: Create and update requests go out as POST and PATCH with their body, and the record
//! that comes back is what the write answers with. Loopback only.

use super::http::{json_response, secret, serve_once};
use super::*;
use crate::cloudflare_dns::client::Client;
use crate::values::cloudflare_dns::{
    Change, NewRecord, Proxy, Record, RecordId, RecordUpdate, Ttl, Zone, ZoneId,
};
use serde_json::json;
use std::time::Duration;
fn zone() -> Zone {
    Zone {
        id: ZoneId::new(ZONE_ID),
        name: "example.com".to_owned(),
    }
}

fn existing() -> Record {
    Record {
        id: RecordId::new(RECORD_ID),
        kind: "A".to_owned(),
        name: "www.example.com".to_owned(),
        content: "203.0.113.5".to_owned(),
        ttl: Ttl::Seconds(300),
        proxy: Proxy::Off,
    }
}

/// The method of the request that applying `change` sends, answered with the record.
fn method_of(change: &Change) -> String {
    let body = json!({
        "success": true,
        "errors": [],
        "result": {"id": RECORD_ID, "type": "A", "name": "www.example.com",
                   "content": "203.0.113.5", "ttl": 300, "proxied": false}
    })
    .to_string();
    let (base, head) = serve_once(json_response("200 OK", &body));
    let mut client = Client::with_base(secret(), &base).expect("client");
    assert!(write::apply(&mut client, change).is_ok());
    let head = head
        .recv_timeout(Duration::from_secs(5))
        .expect("the request head");
    head.split(' ').next().unwrap_or_default().to_owned()
}

#[test]
fn a_new_record_is_posted_and_an_edited_one_is_patched() {
    let create = Change::Create(NewRecord {
        zone: zone(),
        name: "www".to_owned(),
        content: "203.0.113.5".to_owned(),
        ttl: Ttl::Seconds(300),
        proxy: Proxy::Off,
    });
    let update = Change::Update(RecordUpdate {
        zone: zone(),
        record: existing(),
        content: "203.0.113.6".to_owned(),
        ttl: Ttl::Seconds(300),
        proxy: Proxy::Off,
    });
    assert_eq!(method_of(&create), "POST");
    assert_eq!(method_of(&update), "PATCH");
}
