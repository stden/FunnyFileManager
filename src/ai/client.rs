//! Purpose: Blocking HTTPS POST client for Anthropic, `OpenAI`, and Codex APIs.

use std::fs;
use std::io::Read;
use std::time::Duration;

use reqwest::blocking::{Client as HttpClient, ClientBuilder, Response};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use reqwest::redirect::Policy;
use reqwest::Certificate;
use serde_json::Value;
use zeroize::Zeroizing;

use super::config::{AuthHeader, ResolvedProvider};
use crate::values::ai::{AiError, AiProvider};

const TIMEOUT: Duration = Duration::from_secs(60);
const MAX_BODY: u64 = 16 * 1024 * 1024;
const ERROR_PREVIEW: usize = 220;
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Minimal JSON POST transport trait for unit-testing provider adapters without network access.
pub trait AiTransport {
    fn post(&mut self, path: &str, body: &Value) -> Result<Value, AiError>;
}

/// Blocking HTTPS client carrying sensitive auth headers and optional custom gateway headers.
pub struct HttpAiTransport {
    http: HttpClient,
    base_url: String,
    headers: HeaderMap,
}

impl HttpAiTransport {
    pub fn from_resolved(resolved: &ResolvedProvider) -> Result<Self, AiError> {
        Self::from_resolved_with(resolved, |name| std::env::var(name).ok())
    }

    pub fn from_resolved_with(
        resolved: &ResolvedProvider,
        env: impl Fn(&str) -> Option<String>,
    ) -> Result<Self, AiError> {
        let mut builder = HttpClient::builder().danger_accept_invalid_certs(!resolved.tls_verify);
        if let Some(ca) = &resolved.ca_cert {
            let pem = fs::read(ca).map_err(|err| {
                AiError::Config(format!("Cannot read AI CA cert {}: {err}", ca.display()))
            })?;
            let certs = Certificate::from_pem_bundle(&pem)
                .map_err(|err| AiError::Config(format!("Invalid AI CA PEM: {err}")))?;
            if certs.is_empty() {
                return Err(AiError::Config(
                    "Invalid AI CA PEM: no certificates found".to_owned(),
                ));
            }
            for cert in certs {
                builder = builder.add_root_certificate(cert);
            }
        }
        Ok(Self {
            http: build_client(builder)?,
            base_url: resolved.base_url.clone(),
            headers: build_headers(resolved, env)?,
        })
    }

    #[cfg(test)]
    pub(crate) fn with_test_base(
        resolved: &ResolvedProvider,
        env: impl Fn(&str) -> Option<String>,
    ) -> Result<Self, AiError> {
        Ok(Self {
            http: build_client(HttpClient::builder().no_proxy())?,
            base_url: resolved.base_url.clone(),
            headers: build_headers(resolved, env)?,
        })
    }
}

impl AiTransport for HttpAiTransport {
    fn post(&mut self, path: &str, body: &Value) -> Result<Value, AiError> {
        let url = format!("{}{path}", self.base_url);
        let payload = body.to_string().into_bytes();
        let response = self
            .http
            .post(url)
            .headers(self.headers.clone())
            .body(payload)
            .send()
            .map_err(|err| AiError::Network(format!("AI request failed: {err}")))?;
        decode_http_response(response)
    }
}

pub(super) fn build_client(builder: ClientBuilder) -> Result<HttpClient, AiError> {
    builder
        .redirect(Policy::none())
        .timeout(TIMEOUT)
        .user_agent("ffm")
        .build()
        .map_err(|err| AiError::Network(format!("Cannot initialize AI HTTPS client: {err}")))
}

fn build_headers(
    resolved: &ResolvedProvider,
    env: impl Fn(&str) -> Option<String>,
) -> Result<HeaderMap, AiError> {
    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    if resolved.provider == AiProvider::Anthropic {
        headers.insert(
            HeaderName::from_static("anthropic-version"),
            HeaderValue::from_static(ANTHROPIC_VERSION),
        );
    }
    insert_auth_header(&mut headers, resolved)?;
    for (name, template) in &resolved.headers {
        let key = HeaderName::from_bytes(name.as_bytes())
            .map_err(|_err| AiError::Config(format!("Invalid AI header name: {name}")))?;
        let expanded = crate::mcp_remote::config::expand_template(template, &env)
            .map_err(|err| AiError::Config(err.to_string()))?;
        headers.insert(key, parse_header_value(name, expanded.as_str())?);
    }
    Ok(headers)
}

pub(super) fn parse_header_value(name: &str, raw: &str) -> Result<HeaderValue, AiError> {
    let mut val = HeaderValue::from_str(raw)
        .map_err(|_err| AiError::Config(format!("Invalid AI header value for {name}")))?;
    val.set_sensitive(true);
    Ok(val)
}

fn insert_auth_header(headers: &mut HeaderMap, resolved: &ResolvedProvider) -> Result<(), AiError> {
    let (key, raw) = match resolved.auth {
        AuthHeader::AnthropicKey => (
            HeaderName::from_static("x-api-key"),
            Zeroizing::new(resolved.secret.as_str().to_owned()),
        ),
        AuthHeader::Bearer => (
            AUTHORIZATION,
            Zeroizing::new(format!("Bearer {}", resolved.secret.as_str())),
        ),
    };
    let mut val = HeaderValue::from_str(raw.as_str())
        .map_err(|_err| AiError::Config("API key is not a valid HTTP header value".to_owned()))?;
    val.set_sensitive(true);
    headers.insert(key, val);
    Ok(())
}

fn decode_http_response(response: Response) -> Result<Value, AiError> {
    decode_body_limited(response.status().as_u16(), response, MAX_BODY)
}

pub(super) fn decode_body_limited(
    status: u16,
    body: impl Read,
    max_body: u64,
) -> Result<Value, AiError> {
    let mut bytes = Vec::new();
    body.take(max_body.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|err| AiError::Network(format!("Failed reading AI response: {err}")))?;
    if !matches!(u64::try_from(bytes.len()), Ok(len) if len <= max_body) {
        return Err(AiError::InvalidResponse(
            "AI response exceeded size limit".to_owned(),
        ));
    }
    if !(200..300).contains(&status) {
        let preview: String = String::from_utf8_lossy(&bytes)
            .chars()
            .take(ERROR_PREVIEW)
            .collect();
        return Err(AiError::Http(status, preview));
    }
    serde_json::from_slice(&bytes)
        .map_err(|err| AiError::InvalidResponse(format!("Invalid AI JSON response: {err}")))
}
