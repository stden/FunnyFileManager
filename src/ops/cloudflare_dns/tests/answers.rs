//! Purpose: What each answer from the background job does to the dialog: the loads a list starts,
//! the records it fills, the host form it opens, and the failures it shows.

use super::*;
use crate::models::cloudflare_dns::{CloudflareError, CurrentHost, FormField, Target};

#[test]
fn a_zone_list_loads_the_records_of_its_first_zone() {
    let mut panel = CloudflareDnsPanel::new();
    panel.enqueue(Request::Zones);
    publish(&mut panel, Reply::Zones(Ok(zones())));
    assert_eq!(panel.busy, Some(Busy::Records));
    assert_eq!(panel.queued, Some(Request::Records(ZoneId::new(ZONE_A))));
    assert_eq!(panel.zones.len(), 2);
}

#[test]
fn a_record_list_fills_the_list_of_the_zone_it_was_asked_for() {
    let mut panel = loaded();
    panel.enqueue(Request::Records(ZoneId::new(ZONE_A)));
    let records = vec![record("www.example.com", "203.0.113.5")];
    publish(&mut panel, Reply::Records(ZoneId::new(ZONE_A), Ok(records)));
    assert_eq!(panel.records.len(), 1);
    assert_eq!(panel.busy, None);
}

#[test]
fn a_record_list_for_another_zone_is_not_shown() {
    let mut panel = loaded();
    panel.enqueue(Request::Records(ZoneId::new(ZONE_A)));
    let stale = vec![record("other.example.net", "203.0.113.7")];
    publish(&mut panel, Reply::Records(ZoneId::new(ZONE_B), Ok(stale)));
    assert_eq!(
        panel.records.len(),
        1,
        "the list of the selected zone stays"
    );
    assert_eq!(panel.records[0].name, "www.example.com");
}

#[test]
fn the_host_form_is_filled_from_the_matching_zone_and_the_detected_address() {
    let mut panel = loaded();
    let host = CurrentHost {
        hostname: "host.example.com".to_owned(),
        ipv4: Ok("203.0.113.9".to_owned()),
    };
    publish(&mut panel, Reply::Host(Ok(host)));
    let View::Edit(form) = &panel.view else {
        panic!("the host form");
    };
    assert_eq!(form.name, "host");
    assert_eq!(form.content, "203.0.113.9");
    assert_eq!(form.target, Target::New);
    assert_eq!(form.zone.name, "example.com");
}

#[test]
fn a_host_without_a_public_address_keeps_an_empty_editable_content_and_says_why() {
    let mut panel = loaded();
    let host = CurrentHost {
        hostname: "host.example.com".to_owned(),
        ipv4: Err(CloudflareError::Network),
    };
    publish(&mut panel, Reply::Host(Ok(host)));
    let View::Edit(form) = &panel.view else {
        panic!("the host form");
    };
    assert_eq!(form.content, "");
    assert!(form.note.contains("Public IPv4 not detected"));
    assert_eq!(form.field, FormField::Name);
}

#[test]
fn a_host_in_no_zone_is_an_error_that_names_the_hostname() {
    let mut panel = loaded();
    let host = CurrentHost {
        hostname: "host.example.org".to_owned(),
        ipv4: Ok("203.0.113.9".to_owned()),
    };
    publish(&mut panel, Reply::Host(Ok(host)));
    assert_eq!(panel.view, View::Browse);
    assert!(panel.status.contains("host.example.org"));
}

#[test]
fn a_failed_list_shows_its_reason_and_clears_the_loading_line() {
    let mut panel = CloudflareDnsPanel::new();
    panel.enqueue(Request::Zones);
    publish(&mut panel, Reply::Zones(Err(CloudflareError::RateLimited)));
    assert_eq!(panel.busy, None);
    assert_eq!(panel.status, CloudflareError::RateLimited.message());
}
