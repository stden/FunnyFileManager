//! The filter: case-insensitive, over the fields a person searches by, and every
//! machine when it is empty.

use super::machine;
use crate::things::{matches, Machine, OsInfo};

fn web() -> Machine {
    Machine {
        ip_address: Some("10.0.0.5".to_owned()),
        environment: Some("PROD".to_owned()),
        location: Some("Berlin".to_owned()),
        tags: vec!["Database".to_owned()],
        os_info: Some(OsInfo {
            name: "Ubuntu".to_owned(),
            version: "24.04".to_owned(),
        }),
        ..machine("Web-01", "web-01.example.com")
    }
}

#[test]
fn an_empty_filter_matches_every_machine() {
    assert!(matches(&web(), ""));
    assert!(matches(&Machine::default(), ""));
}

#[test]
fn the_filter_ignores_case_in_every_searched_field() {
    let web = web();
    for filter in [
        "web", "WEB-01", "example", "10.0.0", "prod", "berlin", "data",
    ] {
        assert!(matches(&web, filter), "{filter} should match");
    }
}

#[test]
fn a_filter_that_is_in_no_searched_field_does_not_match() {
    assert!(!matches(&web(), "zzz"));
}

#[test]
fn the_operating_system_is_not_a_searched_field() {
    assert!(!matches(&web(), "ubuntu"));
}

#[test]
fn case_folding_follows_unicode() {
    let machine = machine("ÄRGER-01", "host.example.com");
    assert!(matches(&machine, "ärger"));
}
