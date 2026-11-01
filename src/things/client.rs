//! Purpose: The one request the catalog makes, `GET` on the machine list with the
//! token as a bearer credential. HTTPS only, no redirects, bounded in time and
//! size, and every failure reported without the token.

use std::{io::Read, time::Duration};

use reqwest::{
    blocking::{Client as HttpClient, Response},
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
    let client = HttpClient::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .timeout(TIMEOUT)
        .build()
        .map_err(|_http_error| ThingsError::Request)?;
    let response = client
        .get(config.machines_url())
        .headers(auth_headers(config)?)
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
    if response.status() != StatusCode::OK {
        return Err(ThingsError::Request);
    }
    let mut body = Vec::new();
    response
        .take(MAX_BYTES.saturating_add(1))
        .read_to_end(&mut body)
        .map_err(|_io_error| ThingsError::Request)?;
    let size = u64::try_from(body.len()).map_err(|_size_error| ThingsError::Limit)?;
    if size > MAX_BYTES {
        return Err(ThingsError::Limit);
    }
    listing::parse(&body)
}
