//! Purpose: The confirmation text names the zone and the record, and lists only the values that
//! change; a create names the full record name.

use super::*;

#[test]
fn update_confirmation_names_the_zone_record_and_only_the_changed_values() {
    let change = Change::Update(update("203.0.113.9", Ttl::Automatic, Proxy::On));
    assert_eq!(
        change.lines(),
        [
            "Zone: example.com",
            "Record: www.example.com (A)",
            "content: 203.0.113.5 -> 203.0.113.9",
            "TTL: 300 -> automatic",
            "proxied: off -> on",
        ]
    );
}

#[test]
fn update_confirmation_omits_fields_that_stay_the_same() {
    let change = Change::Update(update("203.0.113.5", Ttl::Seconds(600), Proxy::Off));
    assert_eq!(
        change.lines(),
        [
            "Zone: example.com",
            "Record: www.example.com (A)",
            "TTL: 300 -> 600",
        ]
    );
}

#[test]
fn changed_fields_are_only_those_whose_new_value_differs() {
    let edit = update("203.0.113.5", Ttl::Seconds(300), Proxy::Unsupported);
    assert!(
        edit.changed().is_empty(),
        "an unsupported proxy is never a change"
    );
    let edit = update("203.0.113.5", Ttl::Seconds(600), Proxy::Off);
    let changed = edit.changed();
    assert_eq!(changed.content, None);
    assert_eq!(changed.ttl, Some(Ttl::Seconds(600)));
    assert_eq!(changed.proxy, None);
}

#[test]
fn create_confirmation_names_the_full_record_name() {
    let change = Change::Create(NewRecord {
        zone: zone("0000000000000000000000000000000a", "example.com"),
        name: "host".to_owned(),
        content: "203.0.113.5".to_owned(),
        ttl: Ttl::Automatic,
        proxy: Proxy::Off,
    });
    assert_eq!(
        change.lines(),
        [
            "Zone: example.com",
            "New record: host.example.com (A)",
            "content: 203.0.113.5",
            "TTL: automatic",
            "proxied: off",
        ]
    );
}

#[test]
fn a_name_that_already_ends_with_the_zone_is_not_repeated() {
    assert_eq!(
        change::full_name("example.com", "example.com"),
        "example.com"
    );
    assert_eq!(
        change::full_name("a.example.com", "example.com"),
        "a.example.com"
    );
    assert_eq!(change::full_name("host", "example.com"), "host.example.com");
}
