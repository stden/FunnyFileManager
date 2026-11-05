//! Purpose: The zone and record cursors: a zone is selected by its position and found by its id, and
//! a change of zone forgets the records of the zone left behind.

use super::*;

#[test]
fn selecting_a_zone_names_its_id_and_forgets_the_records_of_the_zone_left() {
    let mut panel = CloudflareDnsPanel::new();
    panel.set_zones(vec![
        zone("0000000000000000000000000000000a", "example.com"),
        zone("0000000000000000000000000000000c", "example.net"),
    ]);
    panel.records = vec![record("0000000000000000000000000000000b", "A", Proxy::Off)];
    let id = panel.select_zone(1);
    assert_eq!(id, Some(ZoneId::new("0000000000000000000000000000000c")));
    assert!(
        panel.records.is_empty(),
        "the records of the zone left are forgotten"
    );
    assert_eq!(
        panel.zone_index(&ZoneId::new("0000000000000000000000000000000c")),
        Some(1)
    );
    assert_eq!(
        panel.zone_index(&ZoneId::new("0000000000000000000000000000000f")),
        None
    );
}

#[test]
fn the_selected_record_is_the_one_under_the_cursor_and_none_without_records() {
    let mut panel = CloudflareDnsPanel::new();
    assert!(panel.selected_record().is_none());
    panel.records = vec![
        record("0000000000000000000000000000000b", "A", Proxy::Off),
        record("0000000000000000000000000000000d", "TXT", Proxy::Off),
    ];
    panel.record = 1;
    assert_eq!(
        panel.selected_record().map(|record| record.kind.as_str()),
        Some("TXT")
    );
}

#[test]
fn toggling_the_focus_twice_returns_to_the_zones() {
    let mut panel = CloudflareDnsPanel::new();
    panel.toggle_focus();
    assert_eq!(panel.focus, Focus::Records);
    panel.toggle_focus();
    assert_eq!(panel.focus, Focus::Zones);
}
