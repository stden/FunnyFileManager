//! Purpose: The one request the catalog makes, `GET` on the machine list with the
//! token as a bearer credential. HTTPS only, no redirects, bounded in time and
//! size, and every failure reported without the token.

use std::{io::Read, time::Duration};

use reqwest::{
    blocking::{Client as HttpClient, ClientBuilder, Response},
    header::{HeaderMap, HeaderValue, AUTHORIZATION},
    StatusCode,
};
use zeroize::Zeroizing;

use super::{listing, Config, Listing, ThingsError};

/// How long one request may take, from connecting to the last byte.
const TIMEOUT: Duration = Duration::from_secs(15);
/// The largest machine list read, in bytes: thousands of machines.
const MAX_BYTES: u64 = 8 * 1024 * 1024;

/// The machines the configured catalog lists. Blocks until the answer comes,
/// so it runs on a worker thread.
pub fn fetch(config: &Config) -> Result<Listing, ThingsError> {
    let builder = HttpClient::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .timeout(TIMEOUT);
    request(builder, &config.machines_url(), auth_headers(config)?)
}

fn request(builder: ClientBuilder, url: &str, headers: HeaderMap) -> Result<Listing, ThingsError> {
    let client = builder
        .build()
        .map_err(|_http_error| ThingsError::Request)?;
    let response = client
        .get(url)
        .headers(headers)
        .send()
        .map_err(|_http_error| ThingsError::Request)?;
    read_response(response)
}

/// The `Authorization` header, marked sensitive so no layer logs it.
pub(super) fn auth_headers(config: &Config) -> Result<HeaderMap, ThingsError> {
    let value = Zeroizing::new(format!("Bearer {}", config.token().as_str()));
    let mut header = HeaderValue::from_str(&value).map_err(|_header_error| ThingsError::Request)?;
    header.set_sensitive(true);
    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, header);
    Ok(headers)
}

fn read_response(response: Response) -> Result<Listing, ThingsError> {
    decode_response(response.status(), response)
}

pub(super) fn decode_response(
    status: StatusCode,
    body_reader: impl Read,
) -> Result<Listing, ThingsError> {
    if status != StatusCode::OK {
        return Err(ThingsError::Request);
    }
    let mut body = Vec::new();
    body_reader
        .take(MAX_BYTES.saturating_add(1))
        .read_to_end(&mut body)
        .map_err(|_io_error| ThingsError::Request)?;
    let size = u64::try_from(body.len()).map_err(|_size_error| ThingsError::Limit)?;
    if size > MAX_BYTES {
        return Err(ThingsError::Limit);
    }
    listing::parse(&body)
}

#[cfg(test)]
mod tests;
