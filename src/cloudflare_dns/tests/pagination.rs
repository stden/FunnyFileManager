//! Purpose: Lists read every page the result info names, and refuse an endless listing.

use super::*;
use crate::values::cloudflare_dns::ZoneId;
use serde_json::json;

fn zone_page(name: &str, page: u32, total: u32) -> Result<Value, CloudflareError> {
    Ok(json!({
        "success": true,
        "result": [{"id": ZONE_ID, "name": name}],
        "result_info": {"page": page, "total_pages": total}
    }))
}

#[test]
fn zones_are_read_page_by_page_until_the_last_page() {
    let mut fake = Fake::answering([
        zone_page("example.com", 1, 2),
        zone_page("example.net", 2, 2),
    ]);
    let zones = list::zones(&mut fake).expect("zones");
    let names: Vec<&str> = zones.iter().map(|zone| zone.name.as_str()).collect();
    assert_eq!(names, ["example.com", "example.net"]);
    let pages: Vec<String> = fake
        .calls
        .iter()
        .map(|call| call.query[0].1.clone())
        .collect();
    assert_eq!(pages, ["1", "2"]);
    assert!(fake
        .calls
        .iter()
        .all(|call| call.query.contains(&("per_page", "50".to_owned()))));
}

#[test]
fn records_are_read_a_hundred_to_a_page() {
    let page = Ok(json!({
        "success": true,
        "result": [],
        "result_info": {"page": 1, "total_pages": 1}
    }));
    let mut fake = Fake::answering([page]);
    let records = list::records(&mut fake, &ZoneId::new(ZONE_ID)).expect("records");
    assert_eq!(records, []);
    assert_eq!(fake.calls[0].path, format!("/zones/{ZONE_ID}/dns_records"));
    assert!(fake.calls[0]
        .query
        .contains(&("per_page", "100".to_owned())));
}

#[test]
fn a_failed_page_fails_the_whole_list_without_a_partial_answer() {
    let mut fake = Fake::answering([
        zone_page("example.com", 1, 2),
        Err(CloudflareError::Network),
    ]);
    assert_eq!(list::zones(&mut fake), Err(CloudflareError::Network));
}

#[test]
fn an_endless_listing_stops_at_the_page_limit() {
    let endless = (1..=500).map(|page| zone_page("example.com", page, 1000));
    let mut fake = Fake::answering(endless);
    assert_eq!(list::zones(&mut fake), Err(CloudflareError::Limit));
    assert_eq!(
        fake.calls.len(),
        100,
        "the listing asks for no more than its page limit"
    );
}
