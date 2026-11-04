//! Unit tests for single-adapter footer, empty/unknown-speed `NetInfo` dialog,
//! failed-table network view, dormant adapter colouring, and unreachable probes.

use std::time::{Duration, Instant};

use crossterm::event::KeyCode;

use super::tests::{draw, lines};
use crate::models::tasks::{
    AdapterKind, AdapterSample, InterfaceState, Ipv4, NetworkSample, ProbeOutcome, ProbeTarget,
};
use crate::models::{Modal, Sample, SampleError, TaskList, TaskTab};
use crate::prompt::Privilege;
use crate::test_support::processes::{counters, process, system};
use crate::test_support::task_list::{pid, TaskApp};

fn dormant_sample() -> Sample {
    let dormant = AdapterSample {
        name: "wlan0".to_owned(),
        description: "Wireless".to_owned(),
        kind: AdapterKind::Wifi,
        state: InterfaceState::Dormant,
        ipv4: Some(Ipv4::new(10, 0, 0, 2)),
        link_bps: None,
        rx_bytes: 512,
        tx_bytes: 256,
        rx_packets: 10,
        tx_packets: 5,
        rx_errors: 0,
        tx_errors: 0,
        rx_drops: 0,
        tx_drops: 0,
    };
    let cpu = counters(10, 0, 30);
    Sample {
        system: system(cpu, &[cpu]),
        processes: vec![process(pid(100), "ffm")],
        network: NetworkSample {
            elapsed: Some(Duration::from_secs(10)),
            adapters: vec![dormant],
            gateway: Some(Ipv4::new(10, 0, 0, 1)),
            dns_servers: Vec::new(),
        },
        unreadable: 0,
    }
}

#[test]
fn single_dormant_adapter_unknown_link_speed_and_unreachable_probe_render_cleanly() {
    let mut app = TaskApp::open(Vec::new(), Privilege::User);
    app.apply(dormant_sample());
    let now = Instant::now();
    let targets = app.list().take_probe_request(now).expect("probe targets");
    let results: Vec<(ProbeTarget, ProbeOutcome)> = targets
        .into_iter()
        .map(|target| (target, ProbeOutcome::Unreachable))
        .collect();
    app.list().apply_probe_results(&results, now);

    app.press(KeyCode::Tab);
    app.press(KeyCode::F(2));
    assert_eq!(app.list().tab(), TaskTab::Network);
    let terminal = draw(&mut app, 96, 24);
    let buf = terminal.backend().buffer();
    let screen = lines(buf);
    assert!(
        screen[22].contains("1 adapter, net: TIMEOUT"),
        "{}",
        screen[22]
    );
    assert!(screen.iter().any(|row| row.contains("unreach")));
    assert!(screen
        .iter()
        .any(|row| row.contains("wlan0") && row.contains("WAIT")));

    app.press(KeyCode::F(3));
    assert_eq!(*app.list().modal(), Modal::NetInfo);
    let info_text = lines(draw(&mut app, 96, 24).backend().buffer()).join("\n");
    assert!(info_text.contains("Link speed     -"), "{info_text}");
}

#[test]
fn net_info_without_adapters_and_failed_table_on_network_tab_render_messages() {
    let mut empty = TaskApp::open(Vec::new(), Privilege::User);
    empty.press(KeyCode::Tab);
    empty.list().set_modal(Modal::NetInfo);
    let dialog = lines(draw(&mut empty, 96, 24).backend().buffer()).join("\n");
    assert!(
        dialog.contains("No network interface selected."),
        "{dialog}"
    );

    let mut list = TaskList::default();
    let now = Instant::now();
    let _ = list.take_request(now);
    list.apply_sample(Err(SampleError::WorkerStopped), now);
    list.toggle_tab();
    assert_eq!(list.tab(), TaskTab::Network);
    let mut tracker = crate::mouse::DialogMouse::default();
    let mut terminal =
        ratatui::Terminal::new(crate::test_support::WritableTestBackend::new(96, 24)).unwrap();
    terminal
        .draw(|frame| super::render(frame, &mut tracker, &list))
        .unwrap();
    let failed = lines(terminal.backend().buffer()).join("\n");
    assert!(failed.contains("The process reader stopped."), "{failed}");
}
