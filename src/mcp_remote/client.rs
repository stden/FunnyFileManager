//! Purpose: Streamable HTTP transport for MCP servers with `${ENV_VAR}` sensitive headers.

pub(super) mod response;

use std::fs;
use std::path::Path;
use std::time::Duration;

use reqwest::blocking::{Client as HttpClient, ClientBuilder};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, ACCEPT, CONTENT_TYPE};
use reqwest::redirect::Policy;
use reqwest::{Certificate, Url};
use serde::Serialize;
use serde_json::Value;

use super::config::expand_template;
use crate::values::mcp_remote::{HttpMcpServer, McpRemoteError};
use response::read_rpc_response;

const TIMEOUT: Duration = Duration::from_secs(30);
const SESSION_HEADER: &str = "mcp-session-id";

/// HTTP response envelope returned by an MCP endpoint.
#[derive(Debug, Clone)]
pub struct RpcResponse {
    pub content_type: String,
    pub session_id: Option<String>,
    pub body: String,
}

/// Minimal POST transport trait so Streamable HTTP handshakes remain unit-testable.
pub trait RpcTransport {
    fn post(
        &mut self,
        session_id: Option<&str>,
        body: &Value,
    ) -> Result<RpcResponse, McpRemoteError>;
}

/// Blocking `reqwest` Streamable HTTP client.
pub struct HttpRpcTransport {
    http: HttpClient,
    url: String,
    headers: HeaderMap,
}

impl HttpRpcTransport {
    /// Build a verified HTTPS client, expanding `${ENV}` headers on request start.
    pub fn from_server(server: &HttpMcpServer) -> Result<Self, McpRemoteError> {
        Self::from_server_with(server, |name| std::env::var(name).ok())
    }

    pub fn from_server_with(
        server: &HttpMcpServer,
        env: impl Fn(&str) -> Option<String>,
    ) -> Result<Self, McpRemoteError> {
        validate_https_url(&server.url)?;
        let mut builder = HttpClient::builder().danger_accept_invalid_certs(!server.tls_verify);
        if let Some(ca) = &server.ca_cert {
            builder = add_ca_certs(builder, ca)?;
        }
        Ok(Self {
            http: build_http(builder)?,
            url: server.url.clone(),
            headers: resolved_headers(server, env)?,
        })
    }

    #[cfg(test)]
    pub(crate) fn with_test_url(
        server: &HttpMcpServer,
        env: impl Fn(&str) -> Option<String>,
    ) -> Result<Self, McpRemoteError> {
        Ok(Self {
            http: build_http(HttpClient::builder().no_proxy())?,
            url: server.url.clone(),
            headers: resolved_headers(server, env)?,
        })
    }
}

impl RpcTransport for HttpRpcTransport {
    fn post(
        &mut self,
        session_id: Option<&str>,
        body: &Value,
    ) -> Result<RpcResponse, McpRemoteError> {
        let mut headers = self.headers.clone();
        if let Some(id) = session_id {
            let val = HeaderValue::from_str(id)
                .map_err(|_err| McpRemoteError::Protocol("Invalid Mcp-Session-Id header".into()))?;
            headers.insert(HeaderName::from_static(SESSION_HEADER), val);
        }
        let payload = encode_body(body)?;
        let response = self
            .http
            .post(&self.url)
            .headers(headers)
            .body(payload)
            .send()
            .map_err(|err| McpRemoteError::Network(format!("MCP request failed: {err}")))?;
        read_rpc_response(response)
    }
}

pub(super) fn encode_body<T: Serialize>(body: &T) -> Result<Vec<u8>, McpRemoteError> {
    serde_json::to_vec(body)
        .map_err(|err| McpRemoteError::Protocol(format!("Cannot encode MCP request: {err}")))
}

fn add_ca_certs(builder: ClientBuilder, ca: &Path) -> Result<ClientBuilder, McpRemoteError> {
    let pem = fs::read(ca).map_err(|err| {
        McpRemoteError::Config(format!("Cannot read CA cert {}: {err}", ca.display()))
    })?;
    let certs = Certificate::from_pem_bundle(&pem)
        .map_err(|err| McpRemoteError::Config(format!("Invalid CA PEM: {err}")))?;
    if certs.is_empty() {
        return Err(McpRemoteError::Config(format!(
            "No CA certificates found in {}",
            ca.display()
        )));
    }
    Ok(builder.tls_certs_merge(certs))
}

/// Validate HTTPS endpoint URL without userinfo.
pub fn validate_https_url(raw: &str) -> Result<(), McpRemoteError> {
    let Ok(parsed) = Url::parse(raw.trim()) else {
        return Err(McpRemoteError::Config(format!("Invalid MCP URL: {raw}")));
    };
    if parsed.scheme() != "https" {
        return Err(McpRemoteError::Config(format!(
            "MCP server URL must start with https://: {raw}"
        )));
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(McpRemoteError::Config(
            "MCP URL must not embed credentials".to_owned(),
        ));
    }
    Ok(())
}

pub(super) fn build_http(builder: ClientBuilder) -> Result<HttpClient, McpRemoteError> {
    builder
        .redirect(Policy::none())
        .timeout(TIMEOUT)
        .user_agent("ffm")
        .build()
        .map_err(|err| McpRemoteError::Network(format!("Cannot initialize MCP HTTP client: {err}")))
}

fn resolved_headers(
    server: &HttpMcpServer,
    env: impl Fn(&str) -> Option<String>,
) -> Result<HeaderMap, McpRemoteError> {
    let mut map = HeaderMap::new();
    map.insert(
        ACCEPT,
        HeaderValue::from_static("application/json, text/event-stream"),
    );
    map.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    for (name, template) in &server.headers {
        let key = HeaderName::from_bytes(name.as_bytes())
            .map_err(|_err| McpRemoteError::Config(format!("Invalid MCP header name: {name}")))?;
        let expanded = expand_template(template, &env)?;
        map.insert(key, parse_header_value(name, expanded.as_str())?);
    }
    Ok(map)
}

pub(super) fn parse_header_value(name: &str, raw: &str) -> Result<HeaderValue, McpRemoteError> {
    let mut val = HeaderValue::from_str(raw)
        .map_err(|_err| McpRemoteError::Config(format!("Invalid MCP header value for {name}")))?;
    val.set_sensitive(true);
    Ok(val)
}
