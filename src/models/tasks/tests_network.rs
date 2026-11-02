//! Unit tests for network adapter samples, rate derivation, summary meters,
//! adapter filtering and tab/cursor navigation on `TaskList`.

use std::time::{Duration, Instant};

use crate::models::tasks::{
    AdapterFilter, AdapterKind, AdapterSample, Baselines, BytesPerSec, InterfaceState, Ipv4,
    ListMotion, NetMeter, NetworkSample, Priority, PriorityChange, PriorityClass, SettableClass,
    Snapshot, TaskList, TaskTab,
};
use crate::test_support::processes::{counters, process, sample, system};
use crate::test_support::task_list::pid;

mod tests_more;

fn eth0(rx_bytes: u64, tx_bytes: u64) -> AdapterSample {
    AdapterSample {
        name: "eth0".to_owned(),
        description: "Intel Ethernet".to_owned(),
        kind: AdapterKind::Ethernet,
        state: InterfaceState::Up,
        ipv4: Some(Ipv4::new(192, 168, 1, 10)),
        link_bps: Some(1_000_000_000),
        rx_bytes,
        tx_bytes,
        rx_packets: 100,
        tx_packets: 80,
        rx_errors: 0,
        tx_errors: 0,
        rx_drops: 0,
        tx_drops: 0,
    }
}

fn loopback(rx_bytes: u64) -> AdapterSample {
    AdapterSample {
        name: "lo".to_owned(),
        description: "Loopback".to_owned(),
        kind: AdapterKind::Loopback,
        state: InterfaceState::Up,
        ipv4: Some(Ipv4::new(127, 0, 0, 1)),
        link_bps: None,
        rx_bytes,
        tx_bytes: rx_bytes,
        rx_packets: 10,
        tx_packets: 10,
        rx_errors: 0,
        tx_errors: 0,
        rx_drops: 0,
        tx_drops: 0,
    }
}

#[test]
fn ipv4_and_adapter_labels_parse_and_format() {
    let parsed = Ipv4::parse(" 192.168.0.254 ").unwrap();
    assert_eq!(parsed, Ipv4::new(192, 168, 0, 254));
    assert_eq!(parsed.octets(), [192, 168, 0, 254]);
    assert_eq!(parsed.to_string(), "192.168.0.254");
    assert!(!parsed.is_loopback());
    assert!(!parsed.is_unspecified());
    assert!(Ipv4::new(127, 0, 0, 1).is_loopback());
    assert!(Ipv4::new(0, 0, 0, 0).is_unspecified());
    assert_eq!(AdapterKind::Wifi.label(), "Wi-Fi");
    assert_eq!(InterfaceState::Up.label(), "UP");
    assert!(InterfaceState::Up.is_up());
    assert_eq!(NetworkSample::empty().adapters.len(), 0);

    for bad in ["", "1.2.3", "1.2.3.4.5", "256.0.0.1", "1.2.a.4", "::1"] {
        assert!(Ipv4::parse(bad).is_none(), "expected None for {bad:?}");
    }
}

#[test]
fn bytes_per_sec_computes_rate_and_handles_zero_or_reset_counters() {
    assert_eq!(BytesPerSec::between(100, 500, Duration::ZERO), None);
    assert_eq!(
        BytesPerSec::between(500, 100, Duration::from_secs(1)),
        Some(BytesPerSec::from_bytes_per_sec(0))
    );
    let rate = BytesPerSec::between(1_000, 1_501_000, Duration::from_millis(500)).unwrap();
    assert_eq!(rate.bytes_per_sec(), 3_000_000);
}

#[test]
fn empty_network_sample_leaves_the_header_network_meter_absent() {
    let raw = sample(system(counters(1, 1, 1), &[]), Vec::new());
    let (snap, _) = Snapshot::derive(&Baselines::default(), raw);
    assert_eq!(snap.meters().network(), None);
    assert_eq!(snap.adapters().len(), 0);
}

fn net_snapshot_sample(secs: u64, eth: AdapterSample, lo_bytes: u64) -> crate::models::Sample {
    let mut s = sample(system(counters(secs, 1, secs), &[]), Vec::new());
    s.network = NetworkSample {
        elapsed: Some(Duration::from_secs(secs)),
        adapters: vec![eth, loopback(lo_bytes)],
        gateway: Some(Ipv4::new(192, 168, 1, 1)),
        dns_servers: vec![Ipv4::new(192, 168, 1, 1)],
    };
    s
}

#[test]
fn two_network_samples_derive_adapter_rates_and_ignore_loopback_in_summary_meter() {
    let first = net_snapshot_sample(1, eth0(10_000, 5_000), 9_000_000);
    let (snap1, base1) = Snapshot::derive(&Baselines::default(), first);
    assert_eq!(snap1.meters().network(), Some(NetMeter::Pending));
    assert_eq!(snap1.adapters()[0].rx_rate(), None);

    let mut eth_after = eth0(2_010_000, 505_000);
    eth_after.rx_errors = 1;
    eth_after.rx_drops = 3;
    let second = net_snapshot_sample(3, eth_after, 19_000_000);
    let (snap2, _) = Snapshot::derive(&base1, second);
    let row0 = &snap2.adapters()[0];
    assert_eq!(
        row0.rx_rate(),
        Some(BytesPerSec::from_bytes_per_sec(1_000_000))
    );
    assert_eq!(
        row0.tx_rate(),
        Some(BytesPerSec::from_bytes_per_sec(250_000))
    );
    assert_eq!((row0.total_errors(), row0.total_drops()), (1, 3));
    assert!(row0.is_shown(AdapterFilter::Active));
    let expected = NetMeter::Measured {
        rx: BytesPerSec::from_bytes_per_sec(1_000_000),
        tx: BytesPerSec::from_bytes_per_sec(250_000),
        ceiling: BytesPerSec::from_bytes_per_sec(125_000_000),
        issues: 4,
    };
    assert_eq!(snap2.meters().network(), Some(expected));
}

fn populated_net_list() -> TaskList {
    let mut list = TaskList::default();
    list.toggle_tab();
    let mut down = eth0(0, 0);
    down.name = "eth1".to_owned();
    down.state = InterfaceState::Down;
    let mut raw = sample(system(counters(1, 1, 1), &[]), Vec::new());
    raw.network.adapters = vec![eth0(100, 100), loopback(50), down];
    let now = Instant::now();
    let _ = list.take_request(now);
    list.apply_sample(Ok(raw), now);
    list
}

#[test]
fn tab_filter_and_cursor_navigation_work_on_task_list() {
    let mut list = populated_net_list();
    assert_eq!(list.tab(), TaskTab::Network);
    assert_eq!(list.visible_adapters().len(), 1);
    assert_eq!(list.selected_adapter().unwrap().sample().name, "eth0");

    list.toggle_adapter_filter();
    assert_eq!(list.adapter_filter(), AdapterFilter::All);
    assert_eq!(list.visible_adapters().len(), 3);
    list.select_net_index(2, 1);
    assert_eq!(list.net_offset(), 2);
    list.move_net_cursor(ListMotion::Home, 5);
    assert_eq!(list.net_cursor(), Some(0));
    list.move_net_cursor(ListMotion::End, 5);
    assert_eq!(list.net_cursor(), Some(2));
    list.toggle_adapter_filter();
    assert_eq!(list.net_cursor(), Some(0));
}

#[test]
fn windows_priority_updates_are_recorded_on_process_rows() {
    let mut list = TaskList::default();
    let mut win_proc = process(pid(42), "app.exe");
    win_proc.priority = Priority::Windows {
        class: PriorityClass::Normal,
    };
    let key = win_proc.key;
    let now = Instant::now();
    let _ = list.take_request(now);
    list.apply_sample(
        Ok(sample(system(counters(1, 1, 1), &[]), vec![win_proc])),
        now,
    );
    list.record_priority(key, PriorityChange::Class(SettableClass::High));
    assert_eq!(
        list.row(key).unwrap().sample().priority,
        Priority::Windows {
            class: PriorityClass::High
        }
    );
}
