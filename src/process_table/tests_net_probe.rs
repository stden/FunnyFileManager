//! Unit tests for RFC 1035 DNS packet building and localhost UDP DNS probing.

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket};
use std::time::Duration;

use crate::models::tasks::{Ipv4, NetProber, ProbeOutcome, ProbeTarget};

use super::net_probe::{build_dns_a_query, udp_dns_probe, SystemNetProber};

fn bind_v4_localhost() -> (UdpSocket, SocketAddrV4) {
    let sock = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0)).unwrap();
    sock.set_read_timeout(Some(Duration::from_secs(2)))
        .expect("bounded resolver receive");
    let v4 = match sock.local_addr().unwrap() {
        SocketAddr::V4(addr) => Some(addr),
        SocketAddr::V6(_) => None,
    };
    (sock, v4.expect("IPv4 socket address"))
}

#[test]
fn dns_query_builder_encodes_labels_and_rejects_invalid_domains() {
    let pkt = build_dns_a_query("example.com", 0x1234).expect("valid query");
    assert_eq!(&pkt[..4], &[0x12, 0x34, 0x01, 0x00]);
    assert_eq!(pkt.len(), 29);

    assert!(build_dns_a_query("", 1).is_none());
    assert!(build_dns_a_query("bad..domain", 1).is_none());
}

#[test]
fn udp_dns_probe_measures_rtt_against_local_mock_resolver_and_times_out_when_silent() {
    let (server, server_addr) = bind_v4_localhost();
    let responder = std::thread::spawn(move || {
        let mut req = [0_u8; 512];
        let (n, peer) = server.recv_from(&mut req).unwrap();
        assert!(n >= 12);
        let mut resp = req[..n].to_vec();
        resp[2] |= 0x80;
        server.send_to(&resp, peer).unwrap();
    });

    let ok = udp_dns_probe(
        server_addr,
        "example.com",
        0xabcd,
        Duration::from_millis(500),
    );
    responder.join().unwrap();
    assert!(matches!(ok, ProbeOutcome::Ok(_)), "expected Ok, got {ok:?}");

    let (_silent, silent_addr) = bind_v4_localhost();
    let timed_out = udp_dns_probe(
        silent_addr,
        "example.com",
        0x0001,
        Duration::from_millis(40),
    );
    assert_eq!(timed_out, ProbeOutcome::Timeout);
}

#[test]
fn dns_query_rejects_overlong_labels_and_invalid_name_before_sending() {
    assert!(build_dns_a_query(&"a".repeat(63), 1).is_some());
    assert!(build_dns_a_query(&"a".repeat(64), 1).is_none());
    assert!(build_dns_a_query(&"a".repeat(256), 1).is_none());
    assert!(build_dns_a_query("name.", 1).is_none());
    let (_, server) = bind_v4_localhost();
    assert_eq!(
        udp_dns_probe(server, "bad..name", 1, Duration::from_millis(50)),
        ProbeOutcome::Unreachable
    );
}

#[test]
fn dns_probe_rejects_malformed_and_nonmatching_replies() {
    let txid = 0x1234;
    for reply in [
        vec![0x12, 0x34],
        vec![0x99, 0x34, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        vec![0x12, 0x35, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        vec![0x12, 0x34, 0x00, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    ] {
        let (server, address) = bind_v4_localhost();
        let sender = std::thread::spawn(move || {
            let mut query = [0; 512];
            let (_, peer) = server.recv_from(&mut query).expect("request");
            server.send_to(&reply, peer).expect("reply");
        });
        let outcome = udp_dns_probe(address, "example.com", txid, Duration::from_millis(500));
        sender.join().expect("responder");
        assert_eq!(outcome, ProbeOutcome::Unreachable);
    }
}

#[test]
fn system_prober_rejects_invalid_domains_without_network_access() {
    let mut prober = SystemNetProber::default();
    let target = ProbeTarget::DnsResolve {
        resolver: Ipv4::new(127, 0, 0, 1),
        domain: "bad..name",
    };
    assert_eq!(prober.probe(&[]), Vec::new());
    assert_eq!(
        prober.probe(&[target]),
        vec![(target, ProbeOutcome::Unreachable)]
    );
}

#[cfg(target_os = "linux")]
#[test]
fn udp_probe_rejects_zero_destination_port() {
    let address = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0);
    assert_eq!(
        udp_dns_probe(address, "example.com", 1, Duration::from_secs(1)),
        ProbeOutcome::Unreachable
    );
}
