//! Purpose: Latency and DNS probe targets, bounded rolling RTT/loss history,
//! and plain-sight health classification.

use std::collections::VecDeque;
use std::time::Duration;

use super::super::Percent;
use super::types::Ipv4;

/// Rolling sample window per target for average RTT and packet-loss percentage.
const HISTORY_LEN: usize = 10;
/// RTT threshold at or above which an endpoint is flagged `Slow`.
const SLOW_PING_RTT: Duration = Duration::from_millis(150);
/// DNS lookup threshold at or above which DNS resolution is flagged `Slow`.
const SLOW_DNS_RTT: Duration = Duration::from_millis(250);

/// One network reachability / latency check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeTarget {
    Gateway(Ipv4),
    SystemDns(Ipv4),
    PublicDns {
        name: &'static str,
        ip: Ipv4,
    },
    DnsResolve {
        resolver: Ipv4,
        domain: &'static str,
    },
}

impl ProbeTarget {
    #[must_use]
    pub const fn ip(self) -> Ipv4 {
        match self {
            Self::Gateway(ip)
            | Self::SystemDns(ip)
            | Self::PublicDns { ip, .. }
            | Self::DnsResolve { resolver: ip, .. } => ip,
        }
    }

    #[must_use]
    pub const fn kind_label(self) -> &'static str {
        match self {
            Self::DnsResolve { .. } => "DNS A",
            Self::Gateway(_) | Self::SystemDns(_) | Self::PublicDns { .. } => "Ping",
        }
    }
}

/// Result of one probe attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeOutcome {
    Ok(Duration),
    Timeout,
    Unreachable,
}

/// Plain-sight health diagnosis of a probe row or the whole network.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NetHealth {
    Testing,
    Healthy,
    Slow,
    Lossy,
    Down,
}

impl NetHealth {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Testing => "probing…",
            Self::Healthy => "OK",
            Self::Slow => "SLOW",
            Self::Lossy => "LOSS",
            Self::Down => "TIMEOUT",
        }
    }
}

/// Tracked history and derived health of one `ProbeTarget`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeRow {
    target: ProbeTarget,
    history: VecDeque<ProbeOutcome>,
}

impl ProbeRow {
    pub(super) const fn new(target: ProbeTarget) -> Self {
        Self {
            target,
            history: VecDeque::new(),
        }
    }

    pub(super) fn push(&mut self, outcome: ProbeOutcome) {
        if self.history.len() >= HISTORY_LEN {
            let _ = self.history.pop_front();
        }
        self.history.push_back(outcome);
    }

    #[must_use]
    pub const fn target(&self) -> ProbeTarget {
        self.target
    }

    #[must_use]
    pub fn last(&self) -> Option<ProbeOutcome> {
        self.history.back().copied()
    }

    #[must_use]
    pub fn avg_rtt(&self) -> Option<Duration> {
        let mut total_micros = 0_u128;
        let mut count = 0_u32;
        for item in &self.history {
            if let ProbeOutcome::Ok(rtt) = item {
                total_micros = total_micros.saturating_add(rtt.as_micros());
                count = count.saturating_add(1);
            }
        }
        let n = u128::from(count);
        (n > 0).then(|| {
            let avg = u64::try_from(total_micros / n).unwrap_or(u64::MAX);
            Duration::from_micros(avg)
        })
    }

    #[must_use]
    pub fn loss_percent(&self) -> Percent {
        let sent = u128::try_from(self.history.len()).unwrap_or(u128::MAX);
        let lost_count = self
            .history
            .iter()
            .filter(|o| !matches!(o, ProbeOutcome::Ok(_)))
            .count();
        let lost = u128::try_from(lost_count).unwrap_or(u128::MAX);
        Percent::ratio(lost, sent).unwrap_or(Percent::from_hundredths(0))
    }

    #[must_use]
    pub fn health(&self) -> NetHealth {
        let Some(last) = self.last() else {
            return NetHealth::Testing;
        };
        if !matches!(last, ProbeOutcome::Ok(_)) {
            return NetHealth::Down;
        }
        if self.loss_percent().hundredths() > 0 {
            return NetHealth::Lossy;
        }
        let threshold = match self.target {
            ProbeTarget::DnsResolve { .. } => SLOW_DNS_RTT,
            ProbeTarget::Gateway(_) | ProbeTarget::SystemDns(_) | ProbeTarget::PublicDns { .. } => {
                SLOW_PING_RTT
            }
        };
        if self.avg_rtt().is_some_and(|avg| avg >= threshold) {
            return NetHealth::Slow;
        }
        NetHealth::Healthy
    }
}
