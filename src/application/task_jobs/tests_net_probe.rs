//! Purpose: Tests for the non-blocking network probe worker in `TaskJobs`.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::tests::Fixture;
use crate::models::tasks::{
    AdapterKind, AdapterSample, InterfaceState, Ipv4, NetHealth, NetProber, NetworkSample,
    ProbeOutcome, ProbeTarget,
};
use crate::models::{
    ControlError, Pid, PriorityChange, ProcessHost, ProcessKey, ProcessReader, Sample, SampleError,
};
use crate::test_support::processes::{one_core, process};

struct ProbedHost {
    calls: Arc<Mutex<usize>>,
}

struct OnceReader;

impl ProcessReader for OnceReader {
    fn sample(&mut self) -> Result<Sample, SampleError> {
        let mut s = one_core(vec![process(Pid::new(42).expect("pid"), "ffm")]);
        s.network = NetworkSample {
            elapsed: Some(Duration::from_secs(1)),
            adapters: vec![AdapterSample {
                name: "eth0".to_owned(),
                description: "Ethernet".to_owned(),
                kind: AdapterKind::Ethernet,
                state: InterfaceState::Up,
                ipv4: Some(Ipv4::new(192, 168, 1, 10)),
                link_bps: Some(1_000_000_000),
                rx_bytes: 1_000,
                tx_bytes: 500,
                rx_packets: 10,
                tx_packets: 5,
                rx_errors: 0,
                tx_errors: 0,
                rx_drops: 0,
                tx_drops: 0,
            }],
            gateway: Some(Ipv4::new(192, 168, 1, 1)),
            dns_servers: vec![Ipv4::new(192, 168, 1, 1)],
        };
        Ok(s)
    }
}

struct FastProber {
    calls: Arc<Mutex<usize>>,
}

impl NetProber for FastProber {
    fn probe(&mut self, targets: &[ProbeTarget]) -> Vec<(ProbeTarget, ProbeOutcome)> {
        if let Ok(mut guard) = self.calls.lock() {
            *guard += targets.len();
        }
        targets
            .iter()
            .copied()
            .map(|t| (t, ProbeOutcome::Ok(Duration::from_millis(8))))
            .collect()
    }
}

impl ProcessHost for ProbedHost {
    fn reader(&self) -> Result<Box<dyn ProcessReader>, SampleError> {
        Ok(Box::new(OnceReader))
    }

    fn net_prober(&self) -> Option<Box<dyn NetProber>> {
        Some(Box::new(FastProber {
            calls: Arc::clone(&self.calls),
        }))
    }

    fn signal(&self, _: ProcessKey, _: crate::models::Signal) -> Result<(), ControlError> {
        Ok(())
    }

    fn set_priority(&self, _: ProcessKey, _: PriorityChange) -> Result<(), ControlError> {
        Ok(())
    }
}

#[test]
fn net_probe_worker_runs_off_thread_updates_health_and_records_performance() {
    let calls = Arc::new(Mutex::new(0));
    let host = Arc::new(ProbedHost {
        calls: Arc::clone(&calls),
    });
    let mut fixture = Fixture::new(Some(host));
    fixture.open();
    let t0 = Instant::now();
    fixture.pump(t0);
    // First pump_until_needed applies the sample; next pump starts the prober & sends due batch.
    fixture.pump_until_needed(t0);
    fixture.pump(t0);
    // Wait for probe results to arrive and apply.
    fixture.pump_until_needed(t0 + Duration::from_millis(20));

    assert_eq!(
        fixture.list().net_diagnostics().overall_health(),
        NetHealth::Healthy
    );
    assert!(*calls.lock().expect("lock") >= 3);
    let report = fixture.app.performance.report().join("\n");
    assert!(report.contains("Background/Tasks/NetProbe"), "{report}");

    fixture.app.close_overlay();
    fixture.pump(t0 + Duration::from_millis(30));
}
