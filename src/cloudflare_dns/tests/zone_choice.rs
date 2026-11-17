//! Purpose: A hostname belongs to the zone with the longest name that ends it, on a label boundary.

use super::*;
use crate::values::cloudflare_dns::{Zone, ZoneId};

fn zone(name: &str) -> Zone {
    Zone {
        id: ZoneId::new(ZONE_ID),
        name: name.to_owned(),
    }
}

#[test]
fn a_hostname_belongs_to_the_zone_that_ends_it() {
    let zones = [zone("example.org"), zone("example.com")];
    let chosen = choose(&zones, "host.example.com").map(|zone| zone.name.as_str());
    assert_eq!(chosen, Some("example.com"));
}

#[test]
fn the_longest_matching_zone_wins() {
    let zones = [zone("example.com"), zone("b.example.com")];
    let chosen = choose(&zones, "a.b.example.com").map(|zone| zone.name.as_str());
    assert_eq!(chosen, Some("b.example.com"));
}

#[test]
fn a_zone_matches_only_on_a_label_boundary() {
    let zones = [zone("example.com")];
    assert!(
        choose(&zones, "hostexample.com").is_none(),
        "a suffix inside a label is not the zone"
    );
    assert!(
        choose(&zones, "example.com").is_some(),
        "the apex belongs to its own zone"
    );
}

#[test]
fn case_and_a_trailing_dot_do_not_change_the_match() {
    let zones = [zone("Example.COM")];
    assert!(choose(&zones, "Host.example.com.").is_some());
}

#[test]
fn a_record_name_is_relative_to_its_zone() {
    let example = zone("example.com");
    assert_eq!(relative_name("host.example.com", &example), "host");
    assert_eq!(relative_name("a.b.example.com", &example), "a.b");
    assert_eq!(
        relative_name("example.com", &example),
        "example.com",
        "the apex keeps the zone name"
    );
}

#[test]
fn a_host_outside_the_zone_keeps_its_whole_name() {
    use crate::values::cloudflare_dns::{Zone, ZoneId};
    let zone = Zone {
        id: ZoneId::new("0000000000000000000000000000000a"),
        name: "example.com".to_owned(),
    };
    assert_eq!(relative_name("host.example.net", &zone), "host.example.net");
}
