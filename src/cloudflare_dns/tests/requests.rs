//! Purpose: Each request becomes the one call it names, and its answer is the reply for it.

use super::*;
use crate::models::cloudflare_dns::{Proxy, Record, RecordId, RecordUpdate, Ttl, Zone, ZoneId};
use serde_json::json;

#[test]
fn a_zone_request_reads_the_zone_list_and_answers_with_it() {
    let page = json!({
        "success": true,
        "result": [{"id": ZONE_ID, "name": "example.com"}],
        "result_info": {"page": 1, "total_pages": 1}
    });
    let mut fake = Fake::answering([Ok(page)]);
    let reply = execute(&mut fake, Request::Zones);
    let zone = Zone {
        id: ZoneId::new(ZONE_ID),
        name: "example.com".to_owned(),
    };
    assert_eq!(reply, Reply::Zones(Ok(vec![zone])));
    assert_eq!(fake.calls[0].path, "/zones");
}

#[test]
fn a_record_request_answers_for_the_zone_it_named() {
    let mut fake = Fake::answering([Err(CloudflareError::Network)]);
    let zone = ZoneId::new(ZONE_ID);
    let reply = execute(&mut fake, Request::Records(zone.clone()));
    assert_eq!(reply, Reply::Records(zone, Err(CloudflareError::Network)));
}

#[test]
fn a_write_request_sends_the_patch_of_its_change() {
    let saved = json!({
        "success": true,
        "result": {"id": RECORD_ID, "type": "A", "name": "www.example.com",
                   "content": "203.0.113.9", "ttl": 300, "proxied": false}
    });
    let mut fake = Fake::answering([Ok(saved)]);
    let change = crate::models::cloudflare_dns::Change::Update(RecordUpdate {
        zone: Zone {
            id: ZoneId::new(ZONE_ID),
            name: "example.com".to_owned(),
        },
        record: Record {
            id: RecordId::new(RECORD_ID),
            kind: "A".to_owned(),
            name: "www.example.com".to_owned(),
            content: "203.0.113.5".to_owned(),
            ttl: Ttl::Seconds(300),
            proxy: Proxy::Off,
        },
        content: "203.0.113.9".to_owned(),
        ttl: Ttl::Seconds(300),
        proxy: Proxy::Off,
    });
    let reply = execute(&mut fake, Request::Write(change));
    assert!(matches!(reply, Reply::Written(Ok(_))));
    assert_eq!(fake.calls.len(), 1, "one write, no read");
    assert_eq!(fake.calls[0].method, Method::Patch);
}
