//! Purpose: A PATCH carries only the fields that changed, a create carries the new record, and an
//! applied change is one write request.

use super::*;
use crate::models::cloudflare_dns::{
    Change, NewRecord, Proxy, Record, RecordId, RecordUpdate, Ttl, Zone, ZoneId,
};
use serde_json::json;

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

fn update(content: &str, ttl: Ttl, proxy: Proxy) -> RecordUpdate {
    RecordUpdate {
        zone: zone(),
        record: existing(),
        content: content.to_owned(),
        ttl,
        proxy,
    }
}

fn saved_answer(content: &str) -> Value {
    json!({
        "success": true,
        "result": {"id": RECORD_ID, "type": "A", "name": "www.example.com",
                   "content": content, "ttl": 300, "proxied": false}
    })
}

#[test]
fn a_patch_names_only_the_fields_that_changed() {
    assert_eq!(
        write::patch_body(&update("203.0.113.9", Ttl::Seconds(300), Proxy::Off)),
        json!({"content": "203.0.113.9"})
    );
    assert_eq!(
        write::patch_body(&update("203.0.113.5", Ttl::Seconds(600), Proxy::Off)),
        json!({"ttl": 600})
    );
    assert_eq!(
        write::patch_body(&update("203.0.113.5", Ttl::Automatic, Proxy::On)),
        json!({"ttl": 1, "proxied": true})
    );
}

#[test]
fn a_patch_never_names_the_type_or_the_name() {
    let body = write::patch_body(&update("203.0.113.9", Ttl::Automatic, Proxy::On));
    assert!(body.get("type").is_none());
    assert!(body.get("name").is_none());
}

#[test]
fn a_create_sends_the_type_name_content_ttl_and_proxy() {
    let new = NewRecord {
        zone: zone(),
        name: "host".to_owned(),
        content: "203.0.113.5".to_owned(),
        ttl: Ttl::Automatic,
        proxy: Proxy::Off,
    };
    assert_eq!(
        write::create_body(&new),
        json!({"type": "A", "name": "host", "content": "203.0.113.5", "ttl": 1, "proxied": false})
    );
}

#[test]
fn applying_an_update_sends_one_patch_and_decodes_the_record() {
    let mut fake = Fake::answering([Ok(saved_answer("203.0.113.9"))]);
    let change = Change::Update(update("203.0.113.9", Ttl::Seconds(300), Proxy::Off));
    let saved = write::apply(&mut fake, &change).expect("saved");
    assert_eq!(saved.content, "203.0.113.9");
    assert_eq!(fake.calls.len(), 1);
    assert_eq!(fake.calls[0].method, Method::Patch);
}

#[test]
fn applying_a_create_sends_one_post_to_the_zone() {
    let answer = json!({
        "success": true,
        "result": {"id": RECORD_ID, "type": "A", "name": "host.example.com",
                   "content": "203.0.113.5", "ttl": 1, "proxied": false}
    });
    let mut fake = Fake::answering([Ok(answer)]);
    let change = Change::Create(NewRecord {
        zone: zone(),
        name: "host".to_owned(),
        content: "203.0.113.5".to_owned(),
        ttl: Ttl::Automatic,
        proxy: Proxy::Off,
    });
    assert!(write::apply(&mut fake, &change).is_ok());
    assert_eq!(fake.calls.len(), 1);
    assert_eq!(fake.calls[0].method, Method::Post);
    assert_eq!(fake.calls[0].path, format!("/zones/{ZONE_ID}/dns_records"));
}
