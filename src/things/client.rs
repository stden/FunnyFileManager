//! Purpose: The one request the catalog makes, `GET` on the machine list with the
//! token as a bearer credential. HTTPS only, no redirects, bounded in time and
//! size, and every failure reported without the token.

use std::{io::Read, time::Duration};

use reqwest::{
    blocking::{Client as HttpClient, ClientBuilder, Response},
    header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION, CONTENT_TYPE},
    StatusCode,
};
use serde_json::Value;
use zeroize::Zeroizing;

use super::{listing, Config, Listing, ThingsError};

/// How long one request may take, from connecting to the last byte.
const TIMEOUT: Duration = Duration::from_secs(15);
/// The largest machine list read, in bytes: thousands of machines.
const MAX_BYTES: u64 = 8 * 1024 * 1024;
const SCHEMA_VERSION_HEADER: HeaderName = HeaderName::from_static("graphql-schema-version");

/// Shared HTTPS-only client builder with redirects disabled.
pub(in crate::things) fn https_builder() -> ClientBuilder {
    HttpClient::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .timeout(TIMEOUT)
}

/// The machines the configured catalog lists. Blocks until the answer comes,
/// so it runs on a worker thread.
pub fn fetch(config: &Config) -> Result<Listing, ThingsError> {
    fetch_with(https_builder, config)
}

/// Fetch machines with a caller-supplied HTTP client builder factory.
pub(in crate::things) fn fetch_with(
    make_builder: fn() -> ClientBuilder,
    config: &Config,
) -> Result<Listing, ThingsError> {
    if config.is_graphql() {
        super::graphql::fetch(make_builder(), config)
    } else {
        request(
            make_builder(),
            &config.machines_url(),
            auth_headers(config)?,
        )
    }
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

/// Post one JSON GraphQL request and return the size-bounded HTTP 200 body.
pub(in crate::things) fn post_json(
    builder: ClientBuilder,
    url: &str,
    headers: HeaderMap,
    body: &Value,
) -> Result<Vec<u8>, ThingsError> {
    let payload = serde_json::to_vec(body).map_err(|_json_error| ThingsError::Request)?;
    let client = builder
        .build()
        .map_err(|_http_error| ThingsError::Request)?;
    let response = client
        .post(url)
        .headers(headers)
        .body(payload)
        .send()
        .map_err(|_http_error| ThingsError::Request)?;
    read_limited(response.status(), response)
}

/// Headers required for authenticated Things GraphQL requests.
pub(in crate::things) fn graphql_headers(config: &Config) -> Result<HeaderMap, ThingsError> {
    let mut headers = auth_headers(config)?;
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(SCHEMA_VERSION_HEADER, HeaderValue::from_static("v2-alpha"));
    Ok(headers)
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

pub(super) fn read_limited(
    status: StatusCode,
    body_reader: impl Read,
) -> Result<Vec<u8>, ThingsError> {
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
    Ok(body)
}

pub(super) fn decode_response(
    status: StatusCode,
    body_reader: impl Read,
) -> Result<Listing, ThingsError> {
    let body = read_limited(status, body_reader)?;
    listing::parse(&body)
}

#[cfg(test)]
mod tests;
