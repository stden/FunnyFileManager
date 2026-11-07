//! Purpose: The hostname and public IPv4 a new record is made from. The address comes from the trace
//! page of Cloudflare's public resolver, read without the token.

use super::client::http_client;
use crate::values::cloudflare_dns::{CloudflareError, CurrentHost};
use reqwest::blocking::{Client as HttpClient, ClientBuilder};
#[cfg(test)]
use std::cell::RefCell;
use std::{io::Read, net::Ipv4Addr};

/// The resolver's address. Built from its octets, so no dotted literal is written in the source.
const TRACE_HOST: Ipv4Addr = Ipv4Addr::new(1, 1, 1, 1);
const TRACE_PATH: &str = "/cdn-cgi/trace";
#[cfg(target_os = "linux")]
const LOCAL_HOSTNAME_FILE: &str = "/proc/sys/kernel/hostname";
/// A trace page is a few hundred bytes.
const MAX_TRACE: u64 = 64 * 1024;

/// This machine as the host of a new record: its hostname and its public IPv4.
pub fn detect() -> Result<CurrentHost, CloudflareError> {
    current(local_hostname(), fetch_trace)
}

fn local_hostname() -> Option<String> {
    hostname_from(primary_hostname(), std::env::var("HOSTNAME").ok())
}

#[cfg(windows)]
fn primary_hostname() -> Option<String> {
    std::env::var("COMPUTERNAME").ok()
}

#[cfg(target_os = "linux")]
fn primary_hostname() -> Option<String> {
    std::fs::read_to_string(LOCAL_HOSTNAME_FILE).ok()
}

#[cfg(not(any(windows, target_os = "linux")))]
fn primary_hostname() -> Option<String> {
    std::process::Command::new("hostname")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
}

/// The first of the two sources that is not blank, trimmed.
pub fn hostname_from(primary: Option<String>, fallback: Option<String>) -> Option<String> {
    [primary, fallback]
        .into_iter()
        .flatten()
        .map(|name| name.trim().to_owned())
        .find(|name| !name.is_empty())
}

/// The hostname with its address. An address that cannot be found is kept as its reason, so the
/// form can say why its content is empty; a missing hostname is an error of its own.
pub fn current(
    hostname: Option<String>,
    trace: impl FnOnce() -> Result<String, CloudflareError>,
) -> Result<CurrentHost, CloudflareError> {
    let hostname = hostname.ok_or(CloudflareError::HostName)?;
    let ipv4 = trace()
        .and_then(|body| from_trace(&body))
        .map(|ip| ip.to_string());
    Ok(CurrentHost { hostname, ipv4 })
}

/// The `ip=` line of a trace page, which must be a public IPv4 address.
pub fn from_trace(body: &str) -> Result<Ipv4Addr, CloudflareError> {
    let text = body
        .lines()
        .find_map(|line| line.strip_prefix("ip="))
        .ok_or_else(|| CloudflareError::Invalid("the trace page has no ip line".to_owned()))?;
    let ip = text.trim().parse::<Ipv4Addr>().map_err(|_error| {
        CloudflareError::Invalid("the trace page does not hold an IPv4 address".to_owned())
    })?;
    checked(ip)
}

/// Refuses an address a record must not publish: private, loopback or unspecified.
pub fn checked(ip: Ipv4Addr) -> Result<Ipv4Addr, CloudflareError> {
    if ip.is_private() || ip.is_loopback() || ip.is_unspecified() {
        return Err(CloudflareError::Invalid(format!(
            "{ip} is not a public address"
        )));
    }
    Ok(ip)
}

fn fetch_trace() -> Result<String, CloudflareError> {
    let (builder, url) = trace_endpoint();
    let http: HttpClient = http_client(builder)?;
    let response = http
        .get(url)
        .send()
        .map_err(|_error| CloudflareError::Network)?;
    let status = response.status().as_u16();
    if !(200..=299).contains(&status) {
        return Err(CloudflareError::Status(status));
    }
    let mut bytes = Vec::new();
    response
        .take(MAX_TRACE.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|_error| CloudflareError::Network)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_TRACE {
        return Err(CloudflareError::Limit);
    }
    String::from_utf8(bytes).map_err(|_error| CloudflareError::InvalidResponse)
}

/// The trace page's client builder and address. Production reads it over HTTPS only; a test may
/// point it at a loopback server instead.
pub(super) fn trace_endpoint() -> (ClientBuilder, String) {
    #[cfg(test)]
    if let Some(url) = TEST_TRACE_URL.with(|cell| cell.borrow().clone()) {
        return (HttpClient::builder(), url);
    }
    (
        HttpClient::builder().https_only(true),
        format!("https://{TRACE_HOST}{TRACE_PATH}"),
    )
}

#[cfg(test)]
thread_local! {
    pub(super) static TEST_TRACE_URL: RefCell<Option<String>> = const { RefCell::new(None) };
}
