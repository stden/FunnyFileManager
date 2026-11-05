//! Purpose: The HTTPS client for the Cloudflare API: the bearer token, no redirects, a timeout and
//! a bounded answer. A rate limit is reported, never retried.

use super::call::{Call, Method, Transport};
use super::codec;
use super::token::Token;
use crate::values::cloudflare_dns::CloudflareError;
use reqwest::{
    blocking::{Client as HttpClient, ClientBuilder, Response},
    header::{self, HeaderMap, HeaderValue},
    redirect::Policy,
};
use serde_json::Value;
use std::{io::Read, time::Duration};

/// The only host the token is ever sent to.
const API_BASE: &str = "https://api.cloudflare.com/client/v4";
const TIMEOUT: Duration = Duration::from_secs(15);
const USER_AGENT: &str = "ffm";
/// The largest answer read; a listing of a full page is far below it.
const MAX_BODY: u64 = 2 * 1024 * 1024;

pub struct Client {
    http: HttpClient,
    token: Token,
    base: String,
}

impl Client {
    pub(crate) fn from_environment() -> Result<Self, CloudflareError> {
        let token = Token::from_environment()?;
        let (builder, base) = api_endpoint();
        let http = http_client(builder)?;
        Ok(Self { http, token, base })
    }

    /// A client for `base` over plain HTTP. Loopback tests only: production never has another base.
    #[cfg(test)]
    pub(crate) fn with_base(token: Token, base: &str) -> Result<Self, CloudflareError> {
        Ok(Self {
            http: http_client(HttpClient::builder())?,
            token,
            base: base.to_owned(),
        })
    }

    fn headers(&self) -> Result<HeaderMap, CloudflareError> {
        let mut authorization =
            HeaderValue::from_str(&self.token.authorization()).map_err(|_error| {
                CloudflareError::Invalid(
                    "CLOUDFLARE_API_TOKEN is not a valid header value".to_owned(),
                )
            })?;
        authorization.set_sensitive(true);
        let mut headers = HeaderMap::new();
        headers.insert(header::AUTHORIZATION, authorization);
        headers.insert(header::ACCEPT, HeaderValue::from_static("application/json"));
        Ok(headers)
    }
}

/// An HTTP client that follows no redirect, times out, and identifies itself as `ffm`.
pub fn http_client(builder: ClientBuilder) -> Result<HttpClient, CloudflareError> {
    builder
        .redirect(Policy::none())
        .timeout(TIMEOUT)
        .user_agent(USER_AGENT)
        .build()
        .map_err(|_error| CloudflareError::Network)
}

impl Transport for Client {
    fn send(&mut self, call: &Call) -> Result<Value, CloudflareError> {
        let url = format!("{}{}", self.base, call.path);
        let builder = match call.method {
            Method::Get => self.http.get(url),
            Method::Post => self.http.post(url),
            Method::Patch => self.http.patch(url),
        };
        let mut request = builder.headers(self.headers()?).query(&call.query);
        if let Some(body) = &call.body {
            let bytes =
                serde_json::to_vec(body).map_err(|_error| CloudflareError::InvalidResponse)?;
            request = request
                .header(header::CONTENT_TYPE, "application/json")
                .body(bytes);
        }
        let response = request.send().map_err(|_error| CloudflareError::Network)?;
        read_response(response)
    }
}

/// The decoded answer of a 2xx response. Cloudflare's own error envelope is read first, whatever
/// the status, so the first error it names is the one reported.
fn read_response(response: Response) -> Result<Value, CloudflareError> {
    let status = response.status().as_u16();
    if status == 429 {
        return Err(CloudflareError::RateLimited);
    }
    let mut bytes = Vec::new();
    response
        .take(MAX_BODY.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|_error| CloudflareError::Network)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_BODY {
        return Err(CloudflareError::Limit);
    }
    let value: Option<Value> = serde_json::from_slice(&bytes).ok();
    if let Some(error) = value.as_ref().and_then(codec::error_envelope) {
        return Err(error);
    }
    match (status, value) {
        (200..=299, Some(value)) => Ok(value),
        (200..=299, None) => Err(CloudflareError::InvalidResponse),
        (401 | 403, _) => Err(CloudflareError::AccessDenied),
        (code, _) => Err(CloudflareError::Status(code)),
    }
}

/// The API's client builder and base address. Production talks to the API over HTTPS only; a test
/// may point it at a loopback server instead.
fn api_endpoint() -> (ClientBuilder, String) {
    #[cfg(test)]
    if let Some(base) = TEST_API_BASE.with(|cell| cell.borrow().clone()) {
        return (HttpClient::builder(), base);
    }
    (HttpClient::builder().https_only(true), API_BASE.to_owned())
}

#[cfg(test)]
thread_local! {
    pub(super) static TEST_API_BASE: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}
