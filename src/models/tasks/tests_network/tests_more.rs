//! Purpose: Network navigation and rates at empty and changing interfaces.
use super::*;

#[test]
fn empty_adapter_navigation_stays_empty() {
    let mut list = TaskList::default();
    for motion in [
        ListMotion::Up,
        ListMotion::Down,
        ListMotion::PageUp,
        ListMotion::PageDown,
        ListMotion::Home,
        ListMotion::End,
    ] {
        list.move_net_cursor(motion, 0);
        assert_eq!(list.net_cursor(), None);
        assert_eq!(list.net_offset(), 0);
    }
    list.select_net_index(99, 0);
    assert_eq!(list.selected_adapter(), None);
    assert_eq!(list.net_offset(), 0);
}

#[test]
fn paging_clamps_at_both_ends_and_updates_the_visible_offset() {
    let mut list = populated_net_list();
    list.toggle_adapter_filter();
    list.move_net_cursor(ListMotion::PageDown, 2);
    assert_eq!(list.net_cursor(), Some(2));
    assert_eq!(list.net_offset(), 1);
    list.move_net_cursor(ListMotion::PageUp, 2);
    assert_eq!(list.net_cursor(), Some(0));
    assert_eq!(list.net_offset(), 0);
    list.move_net_cursor(ListMotion::Up, 1);
    assert_eq!(list.net_cursor(), Some(0));
    list.move_net_cursor(ListMotion::End, 1);
    list.move_net_cursor(ListMotion::Down, 1);
    assert_eq!(list.net_cursor(), Some(2));
}

#[test]
fn interface_without_link_speed_uses_measured_traffic_as_scale() {
    let mut first = eth0(0, 0);
    first.link_bps = None;
    let (_, base) = Snapshot::derive(&Baselines::default(), net_snapshot_sample(1, first, 0));
    let mut second = eth0(1_500_000, 0);
    second.link_bps = None;
    let (snapshot, _) = Snapshot::derive(&base, net_snapshot_sample(2, second, 0));
    assert_eq!(
        snapshot.meters().network(),
        Some(NetMeter::Measured {
            rx: BytesPerSec::from_bytes_per_sec(1_500_000),
            tx: BytesPerSec::from_bytes_per_sec(0),
            ceiling: BytesPerSec::from_bytes_per_sec(2_097_152),
            issues: 0,
        })
    );
}

#[test]
fn a_new_interface_after_the_baseline_has_no_fabricated_rate() {
    let first = net_snapshot_sample(1, eth0(0, 0), 0);
    let (_, base) = Snapshot::derive(&Baselines::default(), first);
    let mut second = eth0(100, 100);
    second.name = "new-interface".to_owned();
    let (snapshot, _) = Snapshot::derive(&base, net_snapshot_sample(2, second, 0));
    assert_eq!(snapshot.meters().network(), Some(NetMeter::Pending));
    assert_eq!(snapshot.adapters()[0].rx_rate(), None);
}

#[test]
fn interface_status_labels_cover_waiting_and_unknown_states() {
    assert_eq!(InterfaceState::Dormant.label(), "WAIT");
    assert_eq!(InterfaceState::Unknown.label(), "-");
    assert!(!InterfaceState::Dormant.is_up());
}
