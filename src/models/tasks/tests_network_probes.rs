//! Unit tests for latency and DNS probe targets, rolling RTT/loss metrics,
//! health verdicts and probe scheduling on `TaskList`.

use std::time::{Duration, Instant};

use crate::models::tasks::{
    Ipv4, NetDiagnostics, NetHealth, ProbeOutcome, ProbeRow, ProbeTarget, TaskList,
};
use crate::test_support::processes::{counters, sample, system};

#[test]
fn probe_targets_include_gateway_unique_system_dns_public_dns_and_dns_lookup() {
    let mut diag = NetDiagnostics::default();
    let gw = Ipv4::new(192, 168, 1, 1);
    let sys_dns = Ipv4::new(10, 0, 0, 53);
    diag.sync_endpoints(Some(gw), &[sys_dns, Ipv4::CLOUDFLARE_DNS]);

    let targets: Vec<ProbeTarget> = diag.rows().iter().map(ProbeRow::target).collect();
    assert_eq!(targets[0].ip(), gw);
    assert_eq!(targets[0].kind_label(), "Ping");
    assert_eq!(targets[4].kind_label(), "DNS A");
    assert_eq!(
        targets,
        vec![
            ProbeTarget::Gateway(gw),
            ProbeTarget::SystemDns(sys_dns),
            ProbeTarget::PublicDns {
                name: "Cloudflare",
                ip: Ipv4::CLOUDFLARE_DNS,
            },
            ProbeTarget::PublicDns {
                name: "Google",
                ip: Ipv4::GOOGLE_DNS,
            },
            ProbeTarget::DnsResolve {
                resolver: sys_dns,
                domain: "example.com",
            },
        ]
    );
}

#[test]
fn probe_history_tracks_average_rtt_loss_percent_and_health_verdict() {
    let mut diag = NetDiagnostics::default();
    let gw = Ipv4::new(192, 168, 1, 1);
    diag.sync_endpoints(Some(gw), &[]);
    assert_eq!(diag.overall_health(), NetHealth::Testing);
    assert_eq!(NetHealth::Testing.label(), "probing…");

    let target = ProbeTarget::Gateway(gw);
    let now = Instant::now();
    diag.record_results(&[(target, ProbeOutcome::Ok(Duration::from_millis(4)))], now);
    assert_eq!(
        diag.rows()[0].last(),
        Some(ProbeOutcome::Ok(Duration::from_millis(4)))
    );
    assert_eq!(diag.overall_health(), NetHealth::Healthy);

    diag.record_results(
        &[(target, ProbeOutcome::Ok(Duration::from_millis(296)))],
        now,
    );
    assert_eq!(diag.rows()[0].avg_rtt(), Some(Duration::from_millis(150)));
    assert_eq!(diag.overall_health(), NetHealth::Slow);

    diag.record_results(&[(target, ProbeOutcome::Timeout)], now);
    assert_eq!(diag.overall_health(), NetHealth::Down);

    diag.record_results(
        &[(target, ProbeOutcome::Ok(Duration::from_millis(10)))],
        now,
    );
    assert_eq!(diag.rows()[0].loss_percent().hundredths(), 2_500);
    assert_eq!(diag.rows()[0].health(), NetHealth::Lossy);
}

#[test]
fn task_list_schedules_probes_after_first_sample_and_respects_hold_and_refresh_now() {
    let mut list = TaskList::default();
    let t0 = Instant::now();
    assert!(list.take_probe_request(t0).is_none());

    let mut raw = sample(system(counters(1, 1, 1), &[]), Vec::new());
    raw.network.gateway = Some(Ipv4::new(192, 168, 1, 1));
    let _ = list.take_request(t0);
    list.apply_sample(Ok(raw), t0);

    let batch = list.take_probe_request(t0).expect("due after first sample");
    assert_ne!(batch.len(), 0);
    assert!(list
        .take_probe_request(t0 + Duration::from_secs(1))
        .is_none());

    let ok = [(batch[0], ProbeOutcome::Ok(Duration::from_millis(3)))];
    list.apply_probe_results(&ok, t0 + Duration::from_millis(20));
    assert_eq!(
        list.net_diagnostics().rows()[0].health(),
        NetHealth::Healthy
    );
    assert!(list
        .take_probe_request(t0 + Duration::from_millis(3_050))
        .is_some());

    list.apply_probe_results(&[], t0 + Duration::from_secs(4));
    list.toggle_hold();
    assert!(list
        .take_probe_request(t0 + Duration::from_secs(20))
        .is_none());
    list.refresh_now();
    assert!(list
        .take_probe_request(t0 + Duration::from_secs(20))
        .is_some());
}
