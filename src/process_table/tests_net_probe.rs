//! Unit tests for RFC 1035 DNS packet building and localhost UDP DNS probing.

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket};
use std::time::Duration;

use crate::models::tasks::ProbeOutcome;

use super::net_probe::{build_dns_a_query, udp_dns_probe};

fn bind_v4_localhost() -> (UdpSocket, SocketAddrV4) {
    let sock = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0)).unwrap();
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
