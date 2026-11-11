//! Purpose: The panel refuses a second request while one is outstanding, awaits only its latest
//! request, moves its cursors without losing its place, and answers a failed request for its kind.

use super::*;

#[test]
fn the_panel_refuses_a_second_request_while_one_is_outstanding() {
    let mut panel = CloudflareDnsPanel::new();
    assert!(panel.enqueue(Request::Zones));
    assert!(!panel.enqueue(Request::Host));
    assert_eq!(panel.busy, Some(Busy::Zones));
    let (id, request) = panel.take_queued().expect("the zones request");
    assert_eq!(request, Request::Zones);
    assert!(panel.awaits(id));
    panel.settle();
    assert!(!panel.awaits(id), "a settled wait awaits nothing");
    assert!(panel.enqueue(Request::Host));
}

#[test]
fn an_answer_is_awaited_only_for_the_latest_request() {
    let mut panel = CloudflareDnsPanel::new();
    panel.enqueue(Request::Zones);
    let (first, _) = panel.take_queued().expect("first");
    panel.settle();
    panel.enqueue(Request::Host);
    let (second, _) = panel.take_queued().expect("second");
    assert!(!panel.awaits(first), "the earlier answer is stale");
    assert!(panel.awaits(second));
}

#[test]
fn moving_the_zone_cursor_forgets_the_records_and_names_the_zone_to_load() {
    let mut panel = CloudflareDnsPanel::new();
    panel.set_zones(vec![
        zone("0000000000000000000000000000000a", "example.com"),
        zone("0000000000000000000000000000000c", "example.net"),
    ]);
    panel.set_records(vec![record(
        "0000000000000000000000000000000b",
        "A",
        Proxy::Off,
    )]);
    let loaded = panel.step(true).expect("the second zone is loaded");
    assert_eq!(loaded.as_str(), "0000000000000000000000000000000c");
    assert_eq!(panel.records, []);
    assert_eq!(panel.step(true), None, "the last zone holds");
    assert_eq!(panel.zone, 1);
}

#[test]
fn the_record_cursor_moves_inside_its_own_list_and_stops_at_the_ends() {
    let mut panel = CloudflareDnsPanel::new();
    panel.toggle_focus();
    panel.set_records(vec![
        record("0000000000000000000000000000000b", "A", Proxy::Off),
        record("0000000000000000000000000000000d", "A", Proxy::On),
    ]);
    assert_eq!(panel.step(true), None);
    assert_eq!(panel.record, 1);
    assert_eq!(panel.step(true), None);
    assert_eq!(panel.record, 1);
    panel.set_records(Vec::new());
    assert_eq!(panel.record, 0, "the cursor follows a shorter list");
}

#[test]
fn a_refreshed_zone_list_keeps_the_selected_zone_when_it_is_still_there() {
    let mut panel = CloudflareDnsPanel::new();
    panel.set_zones(vec![
        zone("0000000000000000000000000000000a", "example.com"),
        zone("0000000000000000000000000000000c", "example.net"),
    ]);
    panel.step(true);
    let loaded = panel.set_zones(vec![
        zone("0000000000000000000000000000000c", "example.net"),
        zone("0000000000000000000000000000000a", "example.com"),
    ]);
    assert_eq!(panel.zone, 0);
    assert_eq!(
        loaded.map(|id| id.as_str().to_owned()).as_deref(),
        Some("0000000000000000000000000000000c")
    );
    let gone = panel.set_zones(vec![zone(
        "0000000000000000000000000000000a",
        "example.com",
    )]);
    assert_eq!(panel.zone, 0, "a vanished zone falls back to the first");
    assert!(gone.is_some());
}

#[test]
fn a_failed_request_answers_the_request_it_was_for() {
    let zone_id = ZoneId::new("0000000000000000000000000000000a");
    let reply = Request::Records(zone_id.clone()).fail(CloudflareError::Network);
    assert_eq!(
        reply,
        Reply::Records(zone_id, Err(CloudflareError::Network))
    );
    assert_eq!(
        Request::Host.fail(CloudflareError::Disabled),
        Reply::Host(Err(CloudflareError::Disabled))
    );
}
