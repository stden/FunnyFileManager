//! Purpose: Unit tests for ignoring stale probe outcomes whose targets were removed.

use std::time::{Duration, Instant};

use super::super::probes::{ProbeOutcome, ProbeTarget};
use super::super::types::Ipv4;
use super::NetDiagnostics;

#[test]
fn record_results_ignores_targets_no_longer_in_rows() {
    let mut diag = NetDiagnostics::default();
    let active = Ipv4::new(192, 168, 1, 1);
    let stale = Ipv4::new(10, 0, 0, 1);
    diag.sync_endpoints(Some(active), &[]);

    diag.record_results(
        &[(
            ProbeTarget::Gateway(stale),
            ProbeOutcome::Ok(Duration::from_millis(5)),
        )],
        Instant::now(),
    );

    assert_eq!(diag.rows()[0].last(), None);
}
