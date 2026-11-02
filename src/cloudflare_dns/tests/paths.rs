//! Purpose: Requests address only the zone and record the caller named, with their ids encoded.

use super::*;
use crate::models::cloudflare_dns::{RecordId, ZoneId};
use serde_json::json;

#[test]
fn a_path_segment_keeps_unreserved_characters_and_encodes_the_rest() {
    assert_eq!(call::segment(ZONE_ID), ZONE_ID);
    assert_eq!(call::segment("a/b?c#d e"), "a%2Fb%3Fc%23d%20e");
    assert_eq!(call::segment("é"), "%C3%A9");
}

#[test]
fn a_hostile_zone_id_cannot_leave_its_path_segment() {
    let request = call::records_page(&ZoneId::new("../x?y"), 1);
    assert_eq!(request.path, "/zones/..%2Fx%3Fy/dns_records");
}

#[test]
fn zone_pages_list_fifty_zones_per_page() {
    let request = call::zones_page(2);
    assert_eq!(request.method, Method::Get);
    assert_eq!(request.path, "/zones");
    assert_eq!(
        request.query,
        [("page", "2".to_owned()), ("per_page", "50".to_owned())]
    );
    assert!(request.body.is_none());
}

#[test]
fn record_pages_list_a_hundred_records_per_page() {
    let request = call::records_page(&ZoneId::new(ZONE_ID), 1);
    assert_eq!(request.method, Method::Get);
    assert_eq!(request.path, format!("/zones/{ZONE_ID}/dns_records"));
    assert!(request.query.contains(&("per_page", "100".to_owned())));
}

#[test]
fn a_create_is_a_post_to_the_zone_and_an_update_a_patch_to_one_record() {
    let create = call::create(&ZoneId::new(ZONE_ID), json!({"type": "A"}));
    assert_eq!(create.method, Method::Post);
    assert_eq!(create.path, format!("/zones/{ZONE_ID}/dns_records"));
    assert_eq!(create.body, Some(json!({"type": "A"})));

    let update = call::update(
        &ZoneId::new(ZONE_ID),
        &RecordId::new(RECORD_ID),
        json!({"ttl": 600}),
    );
    assert_eq!(update.method, Method::Patch);
    assert_eq!(
        update.path,
        format!("/zones/{ZONE_ID}/dns_records/{RECORD_ID}")
    );
    assert_eq!(update.body, Some(json!({"ttl": 600})));
}
