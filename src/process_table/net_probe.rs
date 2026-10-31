//! Purpose: Cross-platform `NetProber` implementation performing bounded-timeout
//! reachability/RTT probes and RFC 1035 UDP DNS `A` lookups.

use std::io;
use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use std::time::{Duration, Instant};

use crate::models::tasks::{Ipv4, NetProber, ProbeOutcome, ProbeTarget};

const DEFAULT_TIMEOUT: Duration = Duration::from_millis(750);
const DNS_PORT: u16 = 53;

/// System prober used on the background `"ffm-net-probe"` worker thread.
pub(super) struct SystemNetProber {
    txid_seq: u16,
    timeout: Duration,
}

impl Default for SystemNetProber {
    fn default() -> Self {
        Self {
            txid_seq: 0x464d,
            timeout: DEFAULT_TIMEOUT,
        }
    }
}

impl NetProber for SystemNetProber {
    fn probe(&mut self, targets: &[ProbeTarget]) -> Vec<(ProbeTarget, ProbeOutcome)> {
        let mut results = Vec::with_capacity(targets.len());
        for &target in targets {
            let outcome = match target {
                ProbeTarget::DnsResolve { resolver, domain } => {
                    self.txid_seq = self.txid_seq.wrapping_add(1);
                    let addr = SocketAddrV4::new(Ipv4Addr::from(resolver.octets()), DNS_PORT);
                    udp_dns_probe(addr, domain, self.txid_seq, self.timeout)
                }
                ProbeTarget::Gateway(ip)
                | ProbeTarget::SystemDns(ip)
                | ProbeTarget::PublicDns { ip, .. } => ping_endpoint(ip, self.timeout),
            };
            results.push((target, outcome));
        }
        results
    }
}

#[cfg(windows)]
fn ping_endpoint(ip: Ipv4, timeout: Duration) -> ProbeOutcome {
    crate::platform::icmp_ping(ip, timeout)
}

#[cfg(not(windows))]
fn ping_endpoint(ip: Ipv4, timeout: Duration) -> ProbeOutcome {
    let addr = std::net::SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::from(ip.octets()), DNS_PORT));
    ping_socket(addr, timeout)
}

#[cfg(not(windows))]
fn ping_socket(addr: std::net::SocketAddr, timeout: Duration) -> ProbeOutcome {
    use std::net::TcpStream;
    let start = Instant::now();
    match TcpStream::connect_timeout(&addr, timeout) {
        Ok(_) => ProbeOutcome::Ok(start.elapsed()),
        Err(err) if err.kind() == io::ErrorKind::ConnectionRefused => {
            ProbeOutcome::Ok(start.elapsed())
        }
        Err(err)
            if matches!(
                err.kind(),
                io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
            ) =>
        {
            ProbeOutcome::Timeout
        }
        Err(_) => ProbeOutcome::Unreachable,
    }
}

/// Sends a standard RFC 1035 `A` query to `server` and times the reply.
pub(super) fn udp_dns_probe(
    server: SocketAddrV4,
    domain: &str,
    txid: u16,
    timeout: Duration,
) -> ProbeOutcome {
    let Some(query) = build_dns_a_query(domain, txid) else {
        return ProbeOutcome::Unreachable;
    };
    let Ok(sock) = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)) else {
        return ProbeOutcome::Unreachable;
    };
    drop(sock.set_read_timeout(Some(timeout)));
    drop(sock.set_write_timeout(Some(timeout)));
    let start = Instant::now();
    if sock.send_to(&query, server).is_err() {
        return ProbeOutcome::Unreachable;
    }
    let mut reply = [0_u8; 512];
    match sock.recv_from(&mut reply) {
        Ok((n, _)) if is_matching_dns_reply(&reply, n, txid) => ProbeOutcome::Ok(start.elapsed()),
        Err(err)
            if matches!(
                err.kind(),
                io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
            ) =>
        {
            ProbeOutcome::Timeout
        }
        Ok(_) | Err(_) => ProbeOutcome::Unreachable,
    }
}

pub(super) fn build_dns_a_query(domain: &str, txid: u16) -> Option<Vec<u8>> {
    if domain.is_empty() {
        return None;
    }
    let [hi, lo] = txid.to_be_bytes();
    // Header: ID, Flags=0x0100 (RD), QDCOUNT=1, AN/NS/AR=0.
    let mut pkt = vec![hi, lo, 0x01, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0];
    for label in domain.split('.') {
        let len = u8::try_from(label.len()).ok()?;
        if len == 0 || len > 63 {
            return None;
        }
        pkt.push(len);
        pkt.extend_from_slice(label.as_bytes());
    }
    // Root label 0x00, QTYPE=1 (A), QCLASS=1 (IN).
    pkt.extend_from_slice(&[0x00, 0x00, 0x01, 0x00, 0x01]);
    Some(pkt)
}

fn is_matching_dns_reply(buf: &[u8], len: usize, txid: u16) -> bool {
    let Some(hdr) = buf.get(..len).and_then(|s| s.get(..12)) else {
        return false;
    };
    let [hi, lo] = txid.to_be_bytes();
    hdr.first() == Some(&hi)
        && hdr.get(1) == Some(&lo)
        && hdr.get(2).is_some_and(|flags| (flags & 0x80) != 0)
}

#[cfg(test)]
mod tests;
