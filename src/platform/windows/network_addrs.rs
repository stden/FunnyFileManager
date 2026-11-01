//! Purpose: Discovers per-interface IPv4 addresses, the active default IPv4
//! gateway, and configured IPv4 DNS resolvers via `GetAdaptersAddresses`.

use std::collections::HashMap;

use windows_sys::Win32::Foundation::ERROR_BUFFER_OVERFLOW;
use windows_sys::Win32::NetworkManagement::IpHelper::{
    GetAdaptersAddresses, GAA_FLAG_INCLUDE_GATEWAYS, GAA_FLAG_SKIP_ANYCAST,
    GAA_FLAG_SKIP_MULTICAST, IP_ADAPTER_ADDRESSES_LH, IP_ADAPTER_DNS_SERVER_ADDRESS_XP,
    IP_ADAPTER_GATEWAY_ADDRESS_LH, IP_ADAPTER_UNICAST_ADDRESS_LH,
};
use windows_sys::Win32::NetworkManagement::Ndis::IfOperStatusUp;
use windows_sys::Win32::Networking::WinSock::{AF_INET, SOCKADDR_IN, SOCKET_ADDRESS};

use crate::models::tasks::Ipv4;

/// Discovered IPv4 addresses and routing/DNS endpoints.
#[derive(Default)]
pub struct NetEndpoints {
    pub ipv4_by_index: HashMap<u32, Ipv4>,
    pub gateway: Option<Ipv4>,
    pub dns_servers: Vec<Ipv4>,
}

/// Queries IPv4 unicast, gateway and DNS addresses in one `GetAdaptersAddresses` pass.
#[must_use]
pub fn read_net_endpoints() -> NetEndpoints {
    let Some(buf) = fetch_adapter_buffer() else {
        return NetEndpoints::default();
    };
    let mut out = NetEndpoints::default();
    let mut best_gw_metric = u32::MAX;
    let mut cur = buf.as_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>();
    while !cur.is_null() {
        let item = read_adapter(cur);
        let if_index = adapter_if_index(&item);
        if let Some(ip) = first_unicast_ipv4(item.FirstUnicastAddress.cast_const()) {
            out.ipv4_by_index.insert(if_index, ip);
        }
        if item.OperStatus == IfOperStatusUp {
            record_up_adapter(&item, &mut best_gw_metric, &mut out);
        }
        cur = item.Next.cast_const();
    }
    out
}

fn record_up_adapter(
    item: &IP_ADAPTER_ADDRESSES_LH,
    best_gw_metric: &mut u32,
    out: &mut NetEndpoints,
) {
    if let Some(gw) = first_gateway_ipv4(item.FirstGatewayAddress.cast_const()) {
        if item.Ipv4Metric <= *best_gw_metric {
            *best_gw_metric = item.Ipv4Metric;
            out.gateway = Some(gw);
        }
    }
    collect_dns(
        item.FirstDnsServerAddress.cast_const(),
        &mut out.dns_servers,
    );
}

#[allow(
    unsafe_code,
    reason = "GetAdaptersAddresses fills a caller-allocated byte buffer"
)]
fn fetch_adapter_buffer() -> Option<Vec<u64>> {
    let flags = GAA_FLAG_INCLUDE_GATEWAYS | GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_MULTICAST;
    let family = u32::from(AF_INET);
    let mut size = 32_768_u32;
    for _ in 0..3 {
        let words = usize::try_from(size).ok()?.div_ceil(8).max(1);
        let mut buf = vec![0_u64; words];
        let ptr = buf.as_mut_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>();
        // SAFETY: `buf` is 8-byte aligned and holds at least `size` writable bytes.
        let rc =
            unsafe { GetAdaptersAddresses(family, flags, std::ptr::null(), ptr, &raw mut size) };
        if rc == 0 {
            return Some(buf);
        }
        if rc != ERROR_BUFFER_OVERFLOW {
            return None;
        }
    }
    None
}

#[allow(
    unsafe_code,
    reason = "Dereferences a node in GetAdaptersAddresses's linked list"
)]
fn read_adapter(ptr: *const IP_ADAPTER_ADDRESSES_LH) -> IP_ADAPTER_ADDRESSES_LH {
    // SAFETY: `ptr` points into the live `Vec<u64>` populated by `GetAdaptersAddresses`.
    unsafe { *ptr }
}

#[allow(
    unsafe_code,
    reason = "Reads the IfIndex union field populated by GetAdaptersAddresses"
)]
fn adapter_if_index(item: &IP_ADAPTER_ADDRESSES_LH) -> u32 {
    // SAFETY: `Anonymous1.Anonymous.IfIndex` is initialized for every adapter entry.
    unsafe { item.Anonymous1.Anonymous.IfIndex }
}

#[allow(
    unsafe_code,
    reason = "Walks IP_ADAPTER_UNICAST_ADDRESS_LH linked list"
)]
fn first_unicast_ipv4(mut ptr: *const IP_ADAPTER_UNICAST_ADDRESS_LH) -> Option<Ipv4> {
    while !ptr.is_null() {
        // SAFETY: `ptr` is a valid node in `GetAdaptersAddresses`'s buffer.
        let node = unsafe { *ptr };
        if let Some(ip) = sockaddr_ipv4(node.Address) {
            if !ip.is_unspecified() {
                return Some(ip);
            }
        }
        ptr = node.Next.cast_const();
    }
    None
}

#[allow(
    unsafe_code,
    reason = "Walks IP_ADAPTER_GATEWAY_ADDRESS_LH linked list"
)]
fn first_gateway_ipv4(mut ptr: *const IP_ADAPTER_GATEWAY_ADDRESS_LH) -> Option<Ipv4> {
    while !ptr.is_null() {
        // SAFETY: `ptr` is a valid node in `GetAdaptersAddresses`'s buffer.
        let node = unsafe { *ptr };
        if let Some(ip) = sockaddr_ipv4(node.Address) {
            if !ip.is_unspecified() && !ip.is_loopback() {
                return Some(ip);
            }
        }
        ptr = node.Next.cast_const();
    }
    None
}

#[allow(
    unsafe_code,
    reason = "Walks IP_ADAPTER_DNS_SERVER_ADDRESS_XP linked list"
)]
fn collect_dns(mut ptr: *const IP_ADAPTER_DNS_SERVER_ADDRESS_XP, out: &mut Vec<Ipv4>) {
    while !ptr.is_null() {
        // SAFETY: `ptr` is a valid node in `GetAdaptersAddresses`'s buffer.
        let node = unsafe { *ptr };
        if let Some(ip) = sockaddr_ipv4(node.Address) {
            if !ip.is_unspecified() && !out.contains(&ip) {
                out.push(ip);
            }
        }
        ptr = node.Next.cast_const();
    }
}

#[allow(unsafe_code, reason = "Reads SOCKADDR_IN octets from a SOCKET_ADDRESS")]
fn sockaddr_ipv4(addr: SOCKET_ADDRESS) -> Option<Ipv4> {
    let min_len = i32::try_from(size_of::<SOCKADDR_IN>()).ok()?;
    if addr.lpSockaddr.is_null() || addr.iSockaddrLength < min_len {
        return None;
    }
    // SAFETY: `lpSockaddr` has at least `sizeof(SOCKADDR_IN)` readable bytes.
    let sin = unsafe { addr.lpSockaddr.cast::<SOCKADDR_IN>().read_unaligned() };
    if sin.sin_family != AF_INET {
        return None;
    }
    // SAFETY: `S_un_b` holds the four IPv4 bytes of `IN_ADDR`.
    let b = unsafe { sin.sin_addr.S_un.S_un_b };
    Some(Ipv4::new(b.s_b1, b.s_b2, b.s_b3, b.s_b4))
}
