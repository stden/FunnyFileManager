//! Purpose: Reads Linux network adapter counters and routing/DNS endpoints from
//! `/proc/net/dev`, `/proc/net/route`, `/sys/class/net` and `/etc/resolv.conf`.
//!
//! Pure parsers are separated from file reads so every CI platform tests them.

use std::borrow::Cow;
use std::path::Path;
use std::time::Duration;

use crate::models::tasks::{AdapterKind, AdapterSample, InterfaceState, Ipv4, NetworkSample};

use super::files::Buffer;

const NET_CAP: u64 = 64 * 1024;

/// Reads a `NetworkSample` from `proc_root`, `sys_net_root` and `resolv_path`.
pub(super) fn read_linux_network(
    proc_root: &Path,
    sys_net: &Path,
    resolv_path: &Path,
    elapsed: Duration,
    buffer: &mut Buffer,
) -> NetworkSample {
    let dev_path = proc_root.join("net/dev");
    let dev_text = buffer.read(&dev_path, NET_CAP).ok().map(Cow::into_owned);
    let adapters = dev_text
        .as_deref()
        .map(|text| parse_net_dev(text, sys_net, buffer))
        .unwrap_or_default();
    let route_path = proc_root.join("net/route");
    let gateway = buffer
        .read(&route_path, NET_CAP)
        .ok()
        .and_then(|text| parse_default_gateway(&text));
    let dns_servers = buffer
        .read(resolv_path, NET_CAP)
        .map(|text| parse_resolv_conf(&text))
        .unwrap_or_default();
    if adapters.is_empty() && gateway.is_none() && dns_servers.is_empty() {
        return NetworkSample::empty();
    }
    NetworkSample {
        elapsed: Some(elapsed),
        adapters,
        gateway,
        dns_servers,
    }
}

pub(super) fn parse_net_dev(text: &str, sys_net: &Path, buffer: &mut Buffer) -> Vec<AdapterSample> {
    let mut out = Vec::new();
    for line in text.lines().skip(2) {
        let Some((raw_name, rest)) = line.split_once(':') else {
            continue;
        };
        let name = raw_name.trim();
        if name.is_empty() {
            continue;
        }
        if let Some(sample) = parse_dev_row(name, rest, sys_net, buffer) {
            out.push(sample);
        }
    }
    out
}

fn parse_dev_row(
    name: &str,
    rest: &str,
    sys_net: &Path,
    buffer: &mut Buffer,
) -> Option<AdapterSample> {
    let mut nums = rest.split_whitespace().map(|tok| tok.parse::<u64>().ok());
    let rx_bytes = nums.next()??;
    let rx_packets = nums.next()??;
    let rx_errors = nums.next()??;
    let rx_drops = nums.next()??;
    let (_, _, _, _) = (nums.next()?, nums.next()?, nums.next()?, nums.next()?);
    let tx_bytes = nums.next()??;
    let tx_packets = nums.next()??;
    let tx_errors = nums.next()??;
    let tx_drops = nums.next()??;

    let iface_dir = sys_net.join(name);
    let kind = classify_linux_iface(name, &iface_dir);
    let state = read_operstate(&iface_dir, kind, buffer);
    let link_bps = read_speed_bps(&iface_dir, buffer);
    Some(AdapterSample {
        name: name.to_owned(),
        description: kind.label().to_owned(),
        kind,
        state,
        ipv4: None,
        link_bps,
        rx_bytes,
        tx_bytes,
        rx_packets,
        tx_packets,
        rx_errors,
        tx_errors,
        rx_drops,
        tx_drops,
    })
}

fn classify_linux_iface(name: &str, iface_dir: &Path) -> AdapterKind {
    if name == "lo" {
        return AdapterKind::Loopback;
    }
    if iface_dir.join("wireless").is_dir() || name.starts_with("wl") {
        return AdapterKind::Wifi;
    }
    let virtual_prefixes = [
        "veth",
        "docker",
        "br-",
        "virbr",
        "tun",
        "tap",
        "wg",
        "tailscale",
    ];
    if virtual_prefixes.iter().any(|p| name.starts_with(p)) {
        return AdapterKind::Virtual;
    }
    if name.starts_with("eth") || name.starts_with("en") {
        return AdapterKind::Ethernet;
    }
    AdapterKind::Other
}

fn read_operstate(iface_dir: &Path, kind: AdapterKind, buffer: &mut Buffer) -> InterfaceState {
    let Ok(raw) = buffer.read(&iface_dir.join("operstate"), 64) else {
        return if kind == AdapterKind::Loopback {
            InterfaceState::Up
        } else {
            InterfaceState::Unknown
        };
    };
    match raw.trim() {
        "up" => InterfaceState::Up,
        "down" => InterfaceState::Down,
        "dormant" => InterfaceState::Dormant,
        _ if kind == AdapterKind::Loopback => InterfaceState::Up,
        _ => InterfaceState::Unknown,
    }
}

fn read_speed_bps(iface_dir: &Path, buffer: &mut Buffer) -> Option<u64> {
    let raw = buffer.read(&iface_dir.join("speed"), 64).ok()?;
    let mbps = raw.trim().parse::<u64>().ok()?;
    (mbps > 0 && mbps < u64::from(u32::MAX)).then(|| mbps.saturating_mul(1_000_000))
}

/// Parses `/proc/net/route` for the default IPv4 gateway (`Destination == 0`).
pub(super) fn parse_default_gateway(text: &str) -> Option<Ipv4> {
    for line in text.lines().skip(1) {
        let mut cols = line.split_whitespace();
        let _iface = cols.next()?;
        let dest = cols.next()?;
        let gw_hex = cols.next()?;
        let flags_hex = cols.next()?;
        let flags = u32::from_str_radix(flags_hex, 16).ok()?;
        if dest == "00000000" && (flags & 0x02) != 0 {
            let word = u32::from_str_radix(gw_hex, 16).ok()?;
            let [a, b, c, d] = word.to_le_bytes();
            let ip = Ipv4::new(a, b, c, d);
            if !ip.is_unspecified() {
                return Some(ip);
            }
        }
    }
    None
}

/// Extracts IPv4 `nameserver` entries from `/etc/resolv.conf`.
pub(super) fn parse_resolv_conf(text: &str) -> Vec<Ipv4> {
    let mut out = Vec::new();
    for line in text.lines() {
        let trimmed = line.split('#').next().unwrap_or("").trim();
        let Some(rest) = trimmed.strip_prefix("nameserver") else {
            continue;
        };
        if let Some(ip) = Ipv4::parse(rest) {
            if !ip.is_unspecified() && !out.contains(&ip) {
                out.push(ip);
            }
        }
    }
    out
}
