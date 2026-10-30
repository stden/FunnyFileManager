//! Unit tests for the Network tab (`Tab` / `F12`), summary `Net` meter,
//! Latency & DNS Diagnostics section, `Modal::NetInfo`, and clicks.

use std::time::{Duration, Instant};

use crossterm::event::KeyCode;
use ratatui::layout::Rect;

use super::tests::{draw, fg_of, lines};
use super::{task_click_at, TaskClick};
use crate::models::tasks::{
    AdapterKind, AdapterSample, InterfaceState, Ipv4, NetworkSample, ProbeOutcome, ProbeTarget,
};
use crate::models::{Modal, Sample, TaskTab};
use crate::prompt::Privilege;
use crate::test_support::processes::{counters, process, system};
use crate::test_support::task_list::{pid, TaskApp};
use crate::ui::palette;

fn adapter(
    name: &str,
    kind: AdapterKind,
    state: InterfaceState,
    ipv4: Option<Ipv4>,
    rx_bytes: u64,
    tx_bytes: u64,
    rx_drops: u64,
) -> AdapterSample {
    AdapterSample {
        name: name.to_owned(),
        description: format!("{name} NIC"),
        kind,
        state,
        ipv4,
        link_bps: Some(1_000_000_000),
        rx_bytes,
        tx_bytes,
        rx_packets: 1_200,
        tx_packets: 400,
        rx_errors: 0,
        tx_errors: 0,
        rx_drops,
        tx_drops: 0,
    }
}

fn idle_adapter(name: &str, kind: AdapterKind, state: InterfaceState, drops: u64) -> AdapterSample {
    adapter(name, kind, state, None, 0, 0, drops)
}

fn test_adapters(rx: u64, tx: u64) -> Vec<AdapterSample> {
    let eth_ip = Some(Ipv4::new(192, 168, 1, 10));
    let eth = adapter(
        "Ethernet",
        AdapterKind::Ethernet,
        InterfaceState::Up,
        eth_ip,
        rx,
        tx,
        0,
    );
    let lo = idle_adapter("lo", AdapterKind::Loopback, InterfaceState::Up, 0);
    let wifi = idle_adapter("Wi-Fi", AdapterKind::Wifi, InterfaceState::Down, 2);
    vec![eth, lo, wifi]
}

fn net_sample(secs: u64, rx: u64, tx: u64) -> Sample {
    let cpu = counters(secs, 0, secs * 3);
    Sample {
        system: system(cpu, &[cpu]),
        processes: vec![process(pid(100), "ffm")],
        network: NetworkSample {
            elapsed: Some(Duration::from_secs(secs)),
            adapters: test_adapters(rx, tx),
            gateway: Some(Ipv4::new(192, 168, 1, 1)),
            dns_servers: vec![Ipv4::new(192, 168, 1, 53)],
        },
        unreadable: 0,
    }
}

fn net_app() -> TaskApp {
    let mut app = TaskApp::open(Vec::new(), Privilege::User);
    app.apply(net_sample(10, 1_000_000, 500_000));
    app.apply(net_sample(11, 3_097_152, 756_000));
    let now = Instant::now();
    let targets = app.list().take_probe_request(now).unwrap();
    let results: Vec<(ProbeTarget, ProbeOutcome)> = targets
        .into_iter()
        .map(|t| match t {
            ProbeTarget::Gateway(_) => (t, ProbeOutcome::Ok(Duration::from_millis(4))),
            ProbeTarget::DnsResolve { .. } => (t, ProbeOutcome::Ok(Duration::from_millis(320))),
            _ => (t, ProbeOutcome::Ok(Duration::from_millis(18))),
        })
        .collect();
    app.list().apply_probe_results(&results, now);
    app
}

#[test]
fn summary_header_draws_net_meter_and_probe_health_badge() {
    let mut app = net_app();
    let terminal = draw(&mut app, 100, 24);
    let rows = lines(terminal.backend().buffer());
    assert!(rows[1].contains("Net["), "{}", rows[1]);
    assert!(rows[1].contains("↓2.0M ↑250K"), "{}", rows[1]);
    assert!(rows[2].contains("Net: SLOW"), "{}", rows[2]);
}

#[test]
fn tab_switches_to_network_view_with_adapters_and_diagnostics() {
    let mut app = net_app();
    app.press(KeyCode::Tab);
    assert_eq!(app.list().tab(), TaskTab::Network);
    let terminal = draw(&mut app, 96, 24);
    let screen = lines(terminal.backend().buffer());
    assert!(screen[0].contains("Task list · Network"), "{}", screen[0]);
    assert!(screen
        .iter()
        .any(|l| l.contains("Interface") && l.contains("RX/s")));
    assert!(screen
        .iter()
        .any(|l| l.contains("Ethernet") && l.contains("192.168.1.10")));
    assert!(!screen.iter().any(|l| l.contains("Wi-Fi")));
    assert!(screen
        .iter()
        .any(|l| l.contains("Latency & DNS Diagnostics")));
    assert!(screen
        .iter()
        .any(|l| l.contains("Default gateway") && l.contains("4.0 ms")));
    assert!(screen
        .iter()
        .any(|l| l.contains("DNS example.com") && l.contains("SLOW")));
    assert!(
        screen[22].contains("1 of 3 adapters, net: SLOW"),
        "{}",
        screen[22]
    );
    assert!(screen[23].ends_with("12Proc  "), "{}", screen[23]);
}

#[test]
fn f2_shows_all_adapters_and_highlights_drop_counters_in_red() {
    let mut app = net_app();
    app.press(KeyCode::Tab);
    app.press(KeyCode::F(2));
    let terminal = draw(&mut app, 96, 24);
    let buf = terminal.backend().buffer();
    let wi_fi_row = lines(buf)
        .into_iter()
        .position(|l| l.contains("Wi-Fi"))
        .unwrap();
    assert_eq!(
        fg_of(buf, u16::try_from(wi_fi_row).unwrap(), "0/2"),
        palette::PROMPT_FAILED
    );
}

#[test]
fn net_info_dialog_opens_on_f3_and_clicks_select_adapters_only() {
    let mut app = net_app();
    app.press(KeyCode::F(12));
    app.press(KeyCode::F(2));
    let screen = Rect::new(0, 0, 96, 24);
    assert_eq!(task_click_at(screen, app.list(), 10, 4), None);
    assert_eq!(
        task_click_at(screen, app.list(), 10, 6),
        Some(TaskClick::Row(1))
    );
    assert_eq!(
        task_click_at(screen, app.list(), 92, 23),
        Some(TaskClick::Key(12))
    );

    app.press(KeyCode::F(3));
    assert_eq!(*app.list().modal(), Modal::NetInfo);
    let terminal = draw(&mut app, 96, 24);
    let rendered = lines(terminal.backend().buffer()).join("\n");
    assert!(rendered.contains("Network interface"), "{rendered}");
    assert!(rendered.contains("1000 Mbps"), "{rendered}");
    app.press(KeyCode::Esc);
    assert_eq!(*app.list().modal(), Modal::Browse);
}
