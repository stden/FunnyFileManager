//! The three body shapes Things sends, the fields a listing may leave out, the
//! totals it reports, and the order it keeps.

use crate::things::{parse, Listing, Status, ThingsError};

const WEB: &str = r#"{"id":"1","name":"web-01","hostname":"web-01.example.com","status":"online","tags":["web"]}"#;
const DB: &str =
    r#"{"id":"2","name":"db-01","hostname":"db-01.example.com","status":"maintenance"}"#;

fn listing(body: &str) -> Listing {
    parse(body.as_bytes()).expect("a listing")
}

fn names(listing: &Listing) -> Vec<&str> {
    listing
        .machines
        .iter()
        .map(|machine| machine.name.as_str())
        .collect()
}

#[test]
fn a_bare_array_is_the_whole_listing() {
    let listing = listing(&format!("[{WEB}]"));
    assert_eq!(names(&listing), ["web-01"]);
    assert_eq!(listing.total, None);
}

#[test]
fn the_success_envelope_carries_its_total() {
    let body = format!(
        r#"{{"success":true,"data":[{WEB},{DB}],"total_count":120,"page":1,"per_page":2,"error":null}}"#
    );
    let listing = listing(&body);
    assert_eq!(names(&listing), ["db-01", "web-01"]);
    assert_eq!(listing.total, Some(120));
}

#[test]
fn the_machines_object_is_the_whole_listing() {
    let listing = listing(&format!(r#"{{"machines":[{DB}]}}"#));
    assert_eq!(names(&listing), ["db-01"]);
    assert_eq!(listing.total, None);
}

#[test]
fn fields_the_program_does_not_use_are_ignored() {
    let body = r#"{"success":true,"data":[{"id":"1","name":"web-01","vendor":{"x":[1,2]},"serial":null}],"request_id":"r-1"}"#;
    let listing = listing(body);
    assert_eq!(names(&listing), ["web-01"]);
}

#[test]
fn a_machine_missing_optional_fields_still_lists() {
    let listing = listing(r#"[{"id":"7","name":"bare"}]"#);
    let machine = &listing.machines[0];
    assert_eq!(machine.name, "bare");
    assert_eq!(machine.hostname, "");
    assert_eq!(machine.status, Status::Unknown);
    assert_eq!(machine.tags, Vec::<String>::new());
    assert_eq!(machine.os_info, None);
}

#[test]
fn null_optional_fields_read_as_absent() {
    let listing = listing(
        r#"[{"id":"1","name":"web-01","ip_address":null,"os_info":null,"environment":null,"location":null}]"#,
    );
    let machine = &listing.machines[0];
    assert_eq!(machine.ip_address, None);
    assert_eq!(machine.os_info, None);
    assert_eq!(machine.environment, None);
    assert_eq!(machine.location, None);
}

#[test]
fn a_state_the_program_does_not_know_reads_as_unknown() {
    let listing = listing(
        r#"[{"id":"1","name":"a","status":"retired"},{"id":"2","name":"b","status":"online"}]"#,
    );
    let states: Vec<Status> = listing
        .machines
        .iter()
        .map(|machine| machine.status)
        .collect();
    assert_eq!(states, [Status::Unknown, Status::Online]);
}

#[test]
fn the_listing_is_in_name_order_ignoring_case() {
    let listing = listing(r#"[{"name":"gamma"},{"name":"Alpha"},{"name":"beta"}]"#);
    assert_eq!(names(&listing), ["Alpha", "beta", "gamma"]);
}

#[test]
fn a_malformed_body_is_an_unexpected_response() {
    assert_eq!(parse(b"{").err(), Some(ThingsError::Response));
    assert_eq!(parse(b"\"text\"").err(), Some(ThingsError::Response));
    assert_eq!(parse(br#"{"foo":1}"#).err(), Some(ThingsError::Response));
}

#[test]
fn a_success_envelope_that_says_false_is_an_unexpected_response() {
    assert_eq!(
        parse(br#"{"success":false,"data":[]}"#).err(),
        Some(ThingsError::Response)
    );
}

#[test]
fn a_machine_with_a_wrong_type_fails_the_read_rather_than_being_guessed() {
    assert_eq!(
        parse(br#"[{"id":"1","name":42}]"#).err(),
        Some(ThingsError::Response)
    );
}
