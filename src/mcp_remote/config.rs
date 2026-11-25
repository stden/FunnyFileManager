//! Purpose: Discover Streamable HTTP MCP servers from Claude Code, Kiro, and `ffm.yml`.

mod paths;

#[cfg(test)]
pub use paths::split_config_paths;
pub use paths::{discover, expand_template};

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::values::mcp_remote::{HttpMcpServer, McpRemoteError};

/// Optional `ffm.yml` fields for external MCP JSON configs and inline HTTP MCP servers.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct YamlMcpSettings {
    #[serde(rename = "mcp_configs")]
    pub configs: Vec<PathBuf>,
    #[serde(rename = "mcp_servers")]
    pub servers: BTreeMap<String, RawServer>,
    #[serde(rename = "mcp_tls_verify")]
    pub tls_verify: Option<bool>,
    #[serde(rename = "mcp_ca_cert")]
    pub ca_cert: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct RawServer {
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub url: Option<String>,
    pub headers: BTreeMap<String, String>,
    #[serde(alias = "disabledTools")]
    pub disabled_tools: Vec<String>,
    pub disabled: bool,
    #[serde(alias = "tlsVerify")]
    pub tls_verify: Option<bool>,
    #[serde(alias = "caCert")]
    pub ca_cert: Option<PathBuf>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct McpJsonFile {
    #[serde(rename = "mcpServers")]
    mcp_servers: BTreeMap<String, RawServer>,
}

/// Parse HTTP MCP server entries from a Claude Code / Kiro JSON config string.
pub fn parse_mcp_json(
    text: &str,
    source: &str,
    default_tls_verify: bool,
    default_ca: Option<&Path>,
) -> Result<Vec<HttpMcpServer>, McpRemoteError> {
    let parsed: McpJsonFile = serde_json::from_str(text)
        .map_err(|err| McpRemoteError::Config(format!("Invalid MCP JSON {source}: {err}")))?;
    Ok(parsed
        .mcp_servers
        .into_iter()
        .filter_map(|(name, raw)| to_server(name, raw, source, default_tls_verify, default_ca))
        .collect())
}

pub(super) fn to_server(
    name: String,
    raw: RawServer,
    source: &str,
    default_tls_verify: bool,
    default_ca: Option<&Path>,
) -> Option<HttpMcpServer> {
    if raw.disabled {
        return None;
    }
    let is_http = raw.kind.as_deref().is_some_and(|k| {
        matches!(
            k.to_ascii_lowercase().as_str(),
            "http" | "streamable-http" | "streamablehttp"
        )
    });
    if !is_http {
        return None;
    }
    let url = raw
        .url
        .as_deref()
        .map(str::trim)
        .filter(|u| !u.is_empty())?;
    Some(HttpMcpServer {
        name,
        url: url.trim_end_matches('/').to_owned(),
        headers: raw.headers,
        disabled_tools: raw.disabled_tools,
        source: source.to_owned(),
        tls_verify: raw.tls_verify.unwrap_or(default_tls_verify),
        ca_cert: raw.ca_cert.or_else(|| default_ca.map(Path::to_path_buf)),
    })
}

/// Load optional MCP settings (`mcp_configs`, `mcp_servers`, TLS overrides) from `ffm.yml`.
pub fn load_yaml_settings(path: &Path) -> io::Result<YamlMcpSettings> {
    let Some(stored) = crate::config::document::read_yaml(path)? else {
        return Ok(YamlMcpSettings::default());
    };
    stored.parse()
}
