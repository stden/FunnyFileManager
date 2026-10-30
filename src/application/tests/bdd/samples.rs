//! Portable Windows-style process and network samples, never host snapshots.

use crate::{
    models::tasks::{AdapterKind, AdapterSample, InterfaceState, Ipv4, NetworkSample},
    models::{Pid, Priority, PriorityClass, ProcessSample, Sample},
    test_support::processes::{one_core, process},
};
use std::time::Duration;

pub(super) fn worker(class: PriorityClass) -> ProcessSample {
    ProcessSample {
        priority: Priority::Windows { class },
        ..process(Pid::new(42).unwrap(), "worker")
    }
}

pub(super) fn adapter(name: &str, kind: AdapterKind, state: InterfaceState) -> AdapterSample {
    AdapterSample {
        name: name.into(),
        description: format!("{name} fixture adapter"),
        kind,
        state,
        ipv4: Some(Ipv4::new(192, 0, 2, 10)),
        link_bps: Some(1_000_000_000),
        rx_bytes: 0,
        tx_bytes: 0,
        rx_packets: 0,
        tx_packets: 0,
        rx_errors: 0,
        tx_errors: 0,
        rx_drops: 0,
        tx_drops: 0,
    }
}

pub(super) fn network(seconds: u64, received: u64, transmitted: u64) -> Sample {
    let mut sample = one_core(vec![worker(PriorityClass::Normal)]);
    let mut ethernet = adapter("Ethernet", AdapterKind::Ethernet, InterfaceState::Up);
    ethernet.rx_bytes = received;
    ethernet.tx_bytes = transmitted;
    let mut wifi = adapter("Wi-Fi", AdapterKind::Wifi, InterfaceState::Down);
    wifi.rx_drops = 2;
    sample.network = NetworkSample {
        elapsed: Some(Duration::from_secs(seconds)),
        adapters: vec![
            ethernet,
            wifi,
            adapter("lo", AdapterKind::Loopback, InterfaceState::Up),
        ],
        gateway: Some(Ipv4::new(192, 0, 2, 1)),
        dns_servers: vec![Ipv4::new(192, 0, 2, 53)],
    };
    sample
}

pub(super) fn class(name: &str) -> PriorityClass {
    match name {
        "Idle" => PriorityClass::Idle,
        "BelowNormal" => PriorityClass::BelowNormal,
        "Normal" => PriorityClass::Normal,
        "AboveNormal" => PriorityClass::AboveNormal,
        "High" => PriorityClass::High,
        "Realtime" => PriorityClass::Realtime,
        _ => panic!("unknown fixture class {name}"),
    }
}
