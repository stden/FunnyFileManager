//! Purpose: The checks run before any request: record names, the addresses an `A` or `AAAA` record
//! holds, TTLs, and the proxy rule. Each refusal says what is wrong with the value.

use crate::values::cloudflare_dns::{CloudflareError, Proxy, Ttl};
use std::net::{Ipv4Addr, Ipv6Addr};

/// The longest name, and the longest label, the DNS allows.
const MAX_NAME: usize = 253;
const MAX_LABEL: usize = 63;
/// Cloudflare's TTL range, besides automatic.
const MIN_TTL: u32 = 60;
const MAX_TTL: u32 = 86_400;
/// The record types Cloudflare can proxy.
const PROXIABLE: [&str; 3] = ["A", "AAAA", "CNAME"];

/// A dot-separated list of labels, each of letters, digits and hyphens.
pub fn record_name(text: &str) -> Result<String, CloudflareError> {
    let name = text.trim();
    if name.len() > MAX_NAME {
        return Err(refused("the name is longer than 253 characters"));
    }
    name.split('.')
        .try_for_each(label)
        .map(|()| name.to_owned())
}

/// The value of a record of `kind`: an IPv4 address for `A`, an IPv6 one for `AAAA`, and some text
/// for any other type.
pub fn content(kind: &str, text: &str) -> Result<String, CloudflareError> {
    let value = text.trim();
    match kind {
        "A" => value
            .parse::<Ipv4Addr>()
            .map(|_| value.to_owned())
            .map_err(|_error| refused("an A record needs an IPv4 address")),
        "AAAA" => value
            .parse::<Ipv6Addr>()
            .map(|_| value.to_owned())
            .map_err(|_error| refused("an AAAA record needs an IPv6 address")),
        _ if value.is_empty() || value.chars().any(char::is_control) => {
            Err(refused("the content is empty"))
        }
        _ => Ok(value.to_owned()),
    }
}

/// `1` is automatic; otherwise a number of seconds from 60 to 86400.
pub fn ttl(text: &str) -> Result<Ttl, CloudflareError> {
    match text.trim().parse::<u32>() {
        Ok(1) => Ok(Ttl::Automatic),
        Ok(seconds) if (MIN_TTL..=MAX_TTL).contains(&seconds) => Ok(Ttl::Seconds(seconds)),
        _ => Err(refused("TTL is 1 (automatic) or 60 to 86400 seconds")),
    }
}

/// Only `A`, `AAAA` and `CNAME` records can be proxied.
pub fn proxy(kind: &str, proxy: Proxy) -> Result<(), CloudflareError> {
    if proxy == Proxy::On && !PROXIABLE.contains(&kind) {
        return Err(refused(format!("{kind} records cannot be proxied")));
    }
    Ok(())
}

fn label(label: &str) -> Result<(), CloudflareError> {
    if label.is_empty() {
        return Err(refused("the name has an empty label"));
    }
    if label.len() > MAX_LABEL {
        return Err(refused("a label is longer than 63 characters"));
    }
    if label.starts_with('-') || label.ends_with('-') {
        return Err(refused("a label starts or ends with a hyphen"));
    }
    if !label
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
    {
        return Err(refused(
            "a label has a character other than a letter, a digit or a hyphen",
        ));
    }
    Ok(())
}

fn refused(text: impl Into<String>) -> CloudflareError {
    CloudflareError::Invalid(text.into())
}
