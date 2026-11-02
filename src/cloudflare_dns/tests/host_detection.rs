//! Purpose: The hostname and public IPv4 a new record is made from, and the refusal of an address
//! that is private, loopback or unspecified.

use super::*;
use crate::models::cloudflare_dns::CurrentHost;
use std::net::Ipv4Addr;

const TRACE: &str = "fl=1\nh=example\nip=203.0.113.9\nts=1\n";

#[test]
fn the_hostname_is_the_first_source_that_is_not_blank() {
    assert_eq!(
        host::hostname_from(Some(" web01\n".to_owned()), None).as_deref(),
        Some("web01")
    );
    assert_eq!(
        host::hostname_from(Some("  ".to_owned()), Some("fallback".to_owned())).as_deref(),
        Some("fallback")
    );
    assert_eq!(host::hostname_from(None, None), None);
}

#[test]
fn the_trace_address_is_read_from_its_ip_line() {
    assert_eq!(host::from_trace(TRACE), Ok(Ipv4Addr::new(203, 0, 113, 9)));
    assert!(host::from_trace("fl=1\nts=1\n").is_err(), "no ip line");
    assert!(
        host::from_trace("ip=2001:db8::9\n").is_err(),
        "an IPv6 address is not an IPv4 one"
    );
    assert!(
        host::from_trace("ip=10.0.0.4\n").is_err(),
        "a private address is refused"
    );
}

#[test]
fn private_loopback_and_unspecified_addresses_are_not_public() {
    for ip in [
        Ipv4Addr::new(10, 0, 0, 1),
        Ipv4Addr::new(192, 168, 1, 1),
        Ipv4Addr::new(172, 16, 0, 1),
        Ipv4Addr::LOCALHOST,
        Ipv4Addr::UNSPECIFIED,
    ] {
        assert!(host::checked(ip).is_err(), "{ip} is refused");
    }
    assert_eq!(
        host::checked(Ipv4Addr::new(203, 0, 113, 9)),
        Ok(Ipv4Addr::new(203, 0, 113, 9))
    );
}

#[test]
fn a_known_hostname_is_kept_when_its_address_cannot_be_found() {
    let found = host::current(Some("host.example.com".to_owned()), || Ok(TRACE.to_owned()));
    assert_eq!(
        found,
        Ok(CurrentHost {
            hostname: "host.example.com".to_owned(),
            ipv4: Ok("203.0.113.9".to_owned()),
        })
    );
    let unknown = host::current(Some("host.example.com".to_owned()), || {
        Err(CloudflareError::Network)
    });
    assert_eq!(
        unknown,
        Ok(CurrentHost {
            hostname: "host.example.com".to_owned(),
            ipv4: Err(CloudflareError::Network),
        })
    );
}

#[test]
fn a_missing_hostname_is_an_error_of_its_own() {
    assert_eq!(
        host::current(None, || Ok(TRACE.to_owned())),
        Err(CloudflareError::HostName)
    );
}
