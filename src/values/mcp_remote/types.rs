//! Purpose: HTTP MCP server descriptor, discovered tool metadata, and call output.

use std::collections::BTreeMap;
use std::path::PathBuf;

/// Whether background HTTP MCP requests may reach the network.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum McpAccess {
    #[default]
    Offline,
    Network,
}

/// One Streamable HTTP MCP endpoint loaded from Claude Code, Kiro, or `ffm.yml`.
///
/// Header values hold raw templates (such as `${AGENTGATEWAY_KEY}`), never expanded secrets.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HttpMcpServer {
    pub name: String,
    pub url: String,
    pub headers: BTreeMap<String, String>,
    pub disabled_tools: Vec<String>,
    pub source: String,
    pub tls_verify: bool,
    pub ca_cert: Option<PathBuf>,
}

/// One tool advertised by a remote MCP server's `tools/list`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// Rendered output returned by `tools/call`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpCallResult {
    pub server: String,
    pub tool: String,
    pub is_error: bool,
    pub text: String,
}
