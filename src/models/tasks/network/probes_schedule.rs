//! Purpose: Schedules non-blocking network diagnostic sweeps and maintains
//! the active `ProbeRow` list across endpoint changes.

use std::time::{Duration, Instant};

use super::super::Cadence;
use super::probes::{NetHealth, ProbeOutcome, ProbeRow, ProbeTarget};
use super::types::Ipv4;

/// Interval between background ping/DNS diagnostic sweeps.
pub const PROBE_INTERVAL: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum ProbeSchedule {
    /// Waiting for the first `Sample` so gateway/DNS IPs are known.
    #[default]
    AwaitSample,
    Due,
    InFlight,
    At(Instant),
}

/// Rolling latency & DNS diagnostics owned by `TaskList`.
#[derive(Debug, Clone, Default)]
pub struct NetDiagnostics {
    rows: Vec<ProbeRow>,
    schedule: ProbeSchedule,
    forced: bool,
}

impl NetDiagnostics {
    /// Updates the probe target set from discovered gateway and DNS servers,
    /// preserving history for unchanged targets.
    pub(in crate::models::tasks) fn sync_endpoints(
        &mut self,
        gateway: Option<Ipv4>,
        dns_servers: &[Ipv4],
    ) {
        let desired = build_targets(gateway, dns_servers);
        let mut updated = Vec::with_capacity(desired.len());
        for target in desired {
            let row = self
                .rows
                .iter()
                .find(|existing| existing.target() == target)
                .cloned()
                .unwrap_or_else(|| ProbeRow::new(target));
            updated.push(row);
        }
        self.rows = updated;
        if self.schedule == ProbeSchedule::AwaitSample {
            self.schedule = ProbeSchedule::Due;
        }
    }

    #[must_use]
    pub fn rows(&self) -> &[ProbeRow] {
        &self.rows
    }

    #[must_use]
    pub fn overall_health(&self) -> NetHealth {
        self.rows
            .iter()
            .map(ProbeRow::health)
            .max()
            .unwrap_or(NetHealth::Testing)
    }

    pub(in crate::models::tasks) fn request_now(&mut self) {
        self.forced = true;
        if matches!(self.schedule, ProbeSchedule::At(_)) {
            self.schedule = ProbeSchedule::Due;
        }
    }

    pub(super) fn take_due(&mut self, now: Instant, cadence: Cadence) -> Option<Vec<ProbeTarget>> {
        if self.rows.is_empty() {
            return None;
        }
        let due = match self.schedule {
            ProbeSchedule::Due => cadence == Cadence::Live || self.forced,
            ProbeSchedule::At(at) => (cadence == Cadence::Live && now >= at) || self.forced,
            ProbeSchedule::AwaitSample | ProbeSchedule::InFlight => false,
        };
        if !due {
            return None;
        }
        self.forced = false;
        self.schedule = ProbeSchedule::InFlight;
        Some(self.rows.iter().map(ProbeRow::target).collect())
    }

    pub(in crate::models::tasks) fn record_results(
        &mut self,
        results: &[(ProbeTarget, ProbeOutcome)],
        now: Instant,
    ) {
        for &(target, outcome) in results {
            if let Some(row) = self.rows.iter_mut().find(|r| r.target() == target) {
                row.push(outcome);
            }
        }
        self.schedule = if self.forced {
            ProbeSchedule::Due
        } else {
            ProbeSchedule::At(now + PROBE_INTERVAL)
        };
    }
}

fn build_targets(gateway: Option<Ipv4>, dns_servers: &[Ipv4]) -> Vec<ProbeTarget> {
    let mut out = Vec::with_capacity(5);
    if let Some(gw) = gateway.filter(|ip| !ip.is_unspecified() && !ip.is_loopback()) {
        out.push(ProbeTarget::Gateway(gw));
    }
    let sys_dns = dns_servers
        .iter()
        .copied()
        .find(|ip| !ip.is_unspecified() && !ip.is_loopback());
    if let Some(dns) = sys_dns {
        if gateway != Some(dns) && dns != Ipv4::CLOUDFLARE_DNS && dns != Ipv4::GOOGLE_DNS {
            out.push(ProbeTarget::SystemDns(dns));
        }
    }
    out.push(ProbeTarget::PublicDns {
        name: "Cloudflare",
        ip: Ipv4::CLOUDFLARE_DNS,
    });
    out.push(ProbeTarget::PublicDns {
        name: "Google",
        ip: Ipv4::GOOGLE_DNS,
    });
    let resolver = sys_dns.unwrap_or(Ipv4::CLOUDFLARE_DNS);
    out.push(ProbeTarget::DnsResolve {
        resolver,
        domain: "example.com",
    });
    out
}
