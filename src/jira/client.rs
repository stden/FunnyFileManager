//! Purpose: Blocking HTTPS transport for Jira REST API v2 with sensitive bearer auth.

use std::fs;
use std::io::Read;
use std::time::Duration;

use reqwest::blocking::{Client as HttpClient, ClientBuilder, Response};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use reqwest::redirect::Policy;
use reqwest::Certificate;
use zeroize::Zeroizing;

use super::config::{RuntimeConfig, Token};
use crate::values::jira::JiraError;

const TIMEOUT: Duration = Duration::from_secs(30);
const USER_AGENT: &str = "ffm";
const MAX_BODY: u64 = 32 * 1024 * 1024;
const ERROR_PREVIEW_BYTES: usize = 200;

/// Minimal JSON transport trait so API pagination and error paths stay unit-testable.
pub trait Transport {
    fn get(&mut self, path: &str) -> Result<serde_json::Value, JiraError>;
    fn post(
        &mut self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value, JiraError>;
}

/// Blocking `reqwest` client authenticated with a sensitive bearer token.
pub struct HttpTransport {
    http: HttpClient,
    token: Token,
    base_url: String,
}

impl HttpTransport {
    pub fn new(config: &RuntimeConfig) -> Result<Self, JiraError> {
        let mut builder = HttpClient::builder().danger_accept_invalid_certs(!config.tls_verify);
        if let Some(ca_path) = &config.ca_cert {
            let pem = fs::read(ca_path).map_err(|err| {
                JiraError::Config(format!("Cannot read CA cert {}: {err}", ca_path.display()))
            })?;
            let certs = Certificate::from_pem_bundle(&pem)
                .map_err(|err| JiraError::Config(format!("Invalid PEM CA certificate: {err}")))?;
            if certs.is_empty() {
                return Err(JiraError::Config(
                    "Invalid PEM CA certificate: no certificates found".to_owned(),
                ));
            }
            for cert in certs {
                builder = builder.add_root_certificate(cert);
            }
        }
        Ok(Self {
            http: build_client(builder)?,
            token: config.token.clone(),
            base_url: config.base_url.clone(),
        })
    }

    #[cfg(test)]
    pub(crate) fn with_base(token: Token, base_url: String) -> Result<Self, JiraError> {
        Ok(Self {
            http: build_client(HttpClient::builder().no_proxy())?,
            token,
            base_url,
        })
    }

    fn headers(&self) -> Result<HeaderMap, JiraError> {
        let auth_val = Zeroizing::new(format!("Bearer {}", self.token.as_str()));
        let mut auth = HeaderValue::from_str(&auth_val)
            .map_err(|_err| JiraError::Config("Jira token is not a valid HTTP header".into()))?;
        auth.set_sensitive(true);
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, auth);
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        Ok(headers)
    }

    fn send(
        &self,
        builder: reqwest::blocking::RequestBuilder,
    ) -> Result<serde_json::Value, JiraError> {
        let response = builder
            .headers(self.headers()?)
            .send()
            .map_err(|err| JiraError::Network(format!("Jira request failed: {err}")))?;
        decode_response(response)
    }
}

impl Transport for HttpTransport {
    fn get(&mut self, path: &str) -> Result<serde_json::Value, JiraError> {
        let url = format!("{}{}", self.base_url, path);
        self.send(self.http.get(url))
    }

    fn post(
        &mut self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value, JiraError> {
        let url = format!("{}{}", self.base_url, path);
        let payload = body.to_string().into_bytes();
        self.send(self.http.post(url).body(payload))
    }
}

pub(super) fn build_client(builder: ClientBuilder) -> Result<HttpClient, JiraError> {
    builder
        .redirect(Policy::none())
        .timeout(TIMEOUT)
        .user_agent(USER_AGENT)
        .build()
        .map_err(|err| JiraError::Network(format!("Cannot initialize Jira TLS client: {err}")))
}

fn decode_response(response: Response) -> Result<serde_json::Value, JiraError> {
    decode_response_limited(response.status().as_u16(), response, MAX_BODY)
}

pub(super) fn decode_response_limited(
    status: u16,
    body: impl Read,
    max_body: u64,
) -> Result<serde_json::Value, JiraError> {
    let mut bytes = Vec::new();
    body.take(max_body.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|err| JiraError::Network(format!("Failed reading Jira response: {err}")))?;
    if !matches!(u64::try_from(bytes.len()), Ok(len) if len <= max_body) {
        return Err(JiraError::InvalidResponse(
            "Jira response exceeded size limit".to_owned(),
        ));
    }
    if !(200..300).contains(&status) {
        let preview = String::from_utf8_lossy(&bytes)
            .chars()
            .take(ERROR_PREVIEW_BYTES)
            .collect::<String>();
        return Err(JiraError::Http(status, preview));
    }
    if status == 204 || bytes.is_empty() {
        return Ok(serde_json::Value::Null);
    }
    serde_json::from_slice(&bytes)
        .map_err(|err| JiraError::InvalidResponse(format!("Invalid Jira JSON: {err}")))
}
