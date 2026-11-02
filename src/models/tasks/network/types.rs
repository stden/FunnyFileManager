//! Purpose: Network domain values for `TaskList` — IPv4 octets without
//! `std::net`, checked byte-per-second rates, adapter state and raw samples.

use std::fmt;
use std::time::Duration;

/// An IPv4 address as four octets (`models` may not import `std::net`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ipv4([u8; 4]);

impl Ipv4 {
    pub const CLOUDFLARE_DNS: Self = Self([1, 1, 1, 1]);
    pub const GOOGLE_DNS: Self = Self([8, 8, 8, 8]);

    #[must_use]
    pub const fn new(o0: u8, o1: u8, o2: u8, o3: u8) -> Self {
        Self([o0, o1, o2, o3])
    }

    #[must_use]
    pub const fn octets(self) -> [u8; 4] {
        self.0
    }

    #[must_use]
    pub const fn is_loopback(self) -> bool {
        matches!(self.0, [127, ..])
    }

    #[must_use]
    pub const fn is_unspecified(self) -> bool {
        matches!(self.0, [0, 0, 0, 0])
    }

    /// Parses a dotted-quad IPv4 string such as `"192.168.1.1"`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let mut parts = text.trim().split('.');
        let o0 = parts.next()?.parse::<u8>().ok()?;
        let o1 = parts.next()?.parse::<u8>().ok()?;
        let o2 = parts.next()?.parse::<u8>().ok()?;
        let o3 = parts.next()?.parse::<u8>().ok()?;
        parts.next().is_none().then_some(Self([o0, o1, o2, o3]))
    }
}

impl fmt::Display for Ipv4 {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [o0, o1, o2, o3] = self.0;
        write!(out, "{o0}.{o1}.{o2}.{o3}")
    }
}

/// Throughput in bytes per second, derived with checked integer arithmetic.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct BytesPerSec(u64);

impl BytesPerSec {
    #[must_use]
    pub const fn from_bytes_per_sec(rate: u64) -> Self {
        Self(rate)
    }

    #[must_use]
    pub const fn bytes_per_sec(self) -> u64 {
        self.0
    }

    /// Throughput between two cumulative byte counters separated by `window`.
    /// Returns `None` when `window` is zero; clamps counter resets to `0`.
    #[must_use]
    pub fn between(before: u64, after: u64, window: Duration) -> Option<Self> {
        let micros = window.as_micros();
        if micros == 0 {
            return None;
        }
        let delta = u128::from(after.saturating_sub(before));
        let per_sec = delta.saturating_mul(1_000_000) / micros;
        Some(Self(u64::try_from(per_sec).unwrap_or(u64::MAX)))
    }
}

/// Broad hardware/virtual role of a network interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterKind {
    Ethernet,
    Wifi,
    Loopback,
    Virtual,
    Other,
}

impl AdapterKind {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ethernet => "Ethernet",
            Self::Wifi => "Wi-Fi",
            Self::Loopback => "Loopback",
            Self::Virtual => "Virtual",
            Self::Other => "Other",
        }
    }
}

/// Operational state of a network interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceState {
    Up,
    Down,
    Dormant,
    Unknown,
}

impl InterfaceState {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Up => "UP",
            Self::Down => "DOWN",
            Self::Dormant => "WAIT",
            Self::Unknown => "-",
        }
    }

    #[must_use]
    pub const fn is_up(self) -> bool {
        matches!(self, Self::Up)
    }
}

/// Cumulative counters and metadata of one network adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterSample {
    pub name: String,
    pub description: String,
    pub kind: AdapterKind,
    pub state: InterfaceState,
    pub ipv4: Option<Ipv4>,
    /// Negotiated link speed in bits per second, when reported by the OS.
    pub link_bps: Option<u64>,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_packets: u64,
    pub tx_packets: u64,
    pub rx_errors: u64,
    pub tx_errors: u64,
    pub rx_drops: u64,
    pub tx_drops: u64,
}

/// Network counters and discovered endpoints captured alongside a `Sample`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NetworkSample {
    /// Monotonic time since the reader started, used to compute exact rates.
    pub elapsed: Option<Duration>,
    pub adapters: Vec<AdapterSample>,
    pub gateway: Option<Ipv4>,
    pub dns_servers: Vec<Ipv4>,
}

impl NetworkSample {
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            elapsed: None,
            adapters: Vec::new(),
            gateway: None,
            dns_servers: Vec::new(),
        }
    }
}
