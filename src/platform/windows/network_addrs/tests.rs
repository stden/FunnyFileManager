//! Purpose: Unit tests for `GetAdaptersAddresses` socket, gateway, unicast, and DNS helpers.

use super::*;
use windows_sys::Win32::Networking::WinSock::IN_ADDR;

fn make_sin(family: u16, ip: [u8; 4]) -> SOCKADDR_IN {
    let mut sin = SOCKADDR_IN {
        sin_family: family,
        ..Default::default()
    };
    sin.sin_addr = IN_ADDR {
        S_un: windows_sys::Win32::Networking::WinSock::IN_ADDR_0 {
            S_addr: u32::from_ne_bytes(ip),
        },
    };
    sin
}

fn sock_addr(sin: &mut SOCKADDR_IN) -> SOCKET_ADDRESS {
    SOCKET_ADDRESS {
        lpSockaddr: std::ptr::from_mut(sin).cast(),
        iSockaddrLength: i32::try_from(size_of::<SOCKADDR_IN>()).unwrap(),
    }
}

#[test]
fn sockaddr_ipv4_rejects_null_short_or_ipv6_and_parses_ipv4() {
    let null_addr = SOCKET_ADDRESS {
        lpSockaddr: std::ptr::null_mut(),
        iSockaddrLength: 32,
    };
    assert_eq!(sockaddr_ipv4(null_addr), None);

    let mut v4 = make_sin(AF_INET, [10, 1, 2, 3]);
    let short = SOCKET_ADDRESS {
        lpSockaddr: std::ptr::from_mut(&mut v4).cast(),
        iSockaddrLength: 4,
    };
    assert_eq!(sockaddr_ipv4(short), None);
    assert_eq!(
        sockaddr_ipv4(sock_addr(&mut v4)),
        Some(Ipv4::new(10, 1, 2, 3))
    );

    let mut v6 = make_sin(23, [10, 1, 2, 3]);
    assert_eq!(sockaddr_ipv4(sock_addr(&mut v6)), None);
}

#[test]
fn unicast_gateway_and_dns_walkers_filter_unspecified_loopback_and_duplicates() {
    let mut unspec = make_sin(AF_INET, [0, 0, 0, 0]);
    let mut loop4 = make_sin(AF_INET, [127, 0, 0, 1]);
    let mut gw_sin = make_sin(AF_INET, [192, 168, 1, 1]);

    let uni_zero = IP_ADAPTER_UNICAST_ADDRESS_LH {
        Address: sock_addr(&mut unspec),
        ..Default::default()
    };
    assert_eq!(first_unicast_ipv4(&raw const uni_zero), None);

    let mut gw_valid = IP_ADAPTER_GATEWAY_ADDRESS_LH {
        Address: sock_addr(&mut gw_sin),
        ..Default::default()
    };
    let gw_loop = IP_ADAPTER_GATEWAY_ADDRESS_LH {
        Address: sock_addr(&mut loop4),
        Next: &raw mut gw_valid,
        ..Default::default()
    };
    assert_eq!(
        first_gateway_ipv4(&raw const gw_loop),
        Some(Ipv4::new(192, 168, 1, 1))
    );
    let gw_none = IP_ADAPTER_GATEWAY_ADDRESS_LH {
        Address: sock_addr(&mut unspec),
        ..Default::default()
    };
    assert_eq!(first_gateway_ipv4(&raw const gw_none), None);
}

fn gw_node(sin: &mut SOCKADDR_IN) -> IP_ADAPTER_GATEWAY_ADDRESS_LH {
    IP_ADAPTER_GATEWAY_ADDRESS_LH {
        Address: sock_addr(sin),
        ..Default::default()
    }
}

fn dns_node(
    sin: &mut SOCKADDR_IN,
    next: *mut IP_ADAPTER_DNS_SERVER_ADDRESS_XP,
) -> IP_ADAPTER_DNS_SERVER_ADDRESS_XP {
    IP_ADAPTER_DNS_SERVER_ADDRESS_XP {
        Address: sock_addr(sin),
        Next: next,
        ..Default::default()
    }
}

#[test]
fn record_up_adapter_picks_lowest_metric_gateway_and_deduplicates_dns() {
    let mut dns_ip = make_sin(AF_INET, [8, 8, 8, 8]);
    let mut gw_a = make_sin(AF_INET, [10, 0, 0, 1]);
    let mut gw_b = make_sin(AF_INET, [10, 0, 0, 2]);
    let mut dns2 = dns_node(&mut dns_ip, std::ptr::null_mut());
    let mut dns1 = dns_node(&mut dns_ip, &raw mut dns2);
    let (mut g1, mut g2) = (gw_node(&mut gw_a), gw_node(&mut gw_b));
    let slow = IP_ADAPTER_ADDRESSES_LH {
        Ipv4Metric: 50,
        FirstGatewayAddress: &raw mut g1,
        FirstDnsServerAddress: &raw mut dns1,
        ..Default::default()
    };
    let fast = IP_ADAPTER_ADDRESSES_LH {
        Ipv4Metric: 10,
        FirstGatewayAddress: &raw mut g2,
        ..Default::default()
    };
    let (mut best, mut out) = (u32::MAX, NetEndpoints::default());
    record_up_adapter(&slow, &mut best, &mut out);
    record_up_adapter(&fast, &mut best, &mut out);
    assert_eq!(out.gateway, Some(Ipv4::new(10, 0, 0, 2)));
    assert_eq!(out.dns_servers, vec![Ipv4::new(8, 8, 8, 8)]);
}

#[test]
fn fetch_adapter_buffer_and_walkers_handle_overflow_failure_and_null_addresses() {
    let empty = read_net_endpoints_from(None);
    assert!(empty.ipv4_by_index.is_empty());
    assert_eq!(empty.gateway, None);
    assert_eq!(empty.dns_servers, []);
    assert!(fetch_adapter_buffer_with(8, |_, _| 87).is_none());
    assert!(fetch_adapter_buffer_with(8, |_, size| {
        *size = 64;
        ERROR_BUFFER_OVERFLOW
    })
    .is_none());
    let resized = fetch_adapter_buffer_with(8, |buf, size| {
        *size = 32;
        if buf.len() < 4 {
            ERROR_BUFFER_OVERFLOW
        } else {
            0
        }
    })
    .expect("resized buffer");
    assert_eq!(resized.len(), 4);
    let null_dns = IP_ADAPTER_DNS_SERVER_ADDRESS_XP::default();
    let mut dns_out = Vec::new();
    collect_dns(&raw const null_dns, &mut dns_out);
    assert_eq!(dns_out, []);
    let null_uni = IP_ADAPTER_UNICAST_ADDRESS_LH::default();
    assert_eq!(first_unicast_ipv4(&raw const null_uni), None);
}
