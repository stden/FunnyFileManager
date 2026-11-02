//! Purpose: Exercise the TCP probe through a controlled loopback listener.
#[cfg(not(windows))]
#[test]
fn tcp_probe_uses_a_local_listener_and_rejects_zero_timeout() {
    use super::*;
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("listener");
    let address = listener.local_addr().expect("address");
    assert!(matches!(
        ping_socket(address, Duration::from_secs(1)),
        ProbeOutcome::Ok(_)
    ));
    assert_eq!(
        ping_socket(address, Duration::ZERO),
        ProbeOutcome::Unreachable
    );
}

#[cfg(not(windows))]
#[test]
fn refused_connection_still_proves_the_host_responded() {
    use super::*;

    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("listener");
    let address = listener.local_addr().expect("address");
    drop(listener);
    assert!(matches!(
        ping_socket(address, Duration::from_secs(1)),
        ProbeOutcome::Ok(_)
    ));
}

#[cfg(not(windows))]
#[test]
fn no_time_budget_rejects_all_ping_targets_without_waiting() {
    use super::*;

    let ip = Ipv4::new(127, 0, 0, 1);
    let targets = [
        ProbeTarget::Gateway(ip),
        ProbeTarget::SystemDns(ip),
        ProbeTarget::PublicDns { name: "local", ip },
    ];
    let mut prober = SystemNetProber {
        txid_seq: 0,
        timeout: Duration::ZERO,
    };
    let results = prober.probe(&targets);
    assert_eq!(results.len(), targets.len());
    assert!(results
        .iter()
        .all(|(_, outcome)| *outcome == ProbeOutcome::Unreachable));
}

#[test]
fn system_prober_loopback_and_silent_udp_dns_timeout_complete_boundedly() {
    use super::*;
    use std::net::{SocketAddr, UdpSocket};

    let ip = Ipv4::new(127, 0, 0, 1);
    let mut prober = SystemNetProber {
        txid_seq: 7,
        timeout: Duration::from_millis(50),
    };
    let out = prober.probe(&[ProbeTarget::Gateway(ip), ProbeTarget::SystemDns(ip)]);
    assert_eq!(out.len(), 2);

    let silent = UdpSocket::bind(("127.0.0.1", 0)).expect("udp bind");
    let SocketAddr::V4(server) = silent.local_addr().expect("addr") else {
        panic!("ipv4");
    };
    assert_eq!(
        udp_dns_probe(server, "example.com", 42, Duration::from_millis(25)),
        ProbeOutcome::Timeout
    );
    assert_eq!(
        udp_dns_probe(server, "", 42, Duration::from_millis(25)),
        ProbeOutcome::Unreachable
    );
}
