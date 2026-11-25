//! Purpose: HTTP MCP background requests, replies, and typed failures.

use std::fmt;

use super::{HttpMcpServer, McpCallResult, McpTool};

/// What the HTTP MCP background worker is currently doing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum McpBusy {
    ListingTools,
    CallingTool,
}

impl McpBusy {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::ListingTools => "Loading MCP tools...",
            Self::CallingTool => "Calling MCP tool...",
        }
    }
}

/// Request dispatched to the HTTP MCP worker.
#[derive(Debug, Clone)]
pub enum McpRequest {
    ListTools(HttpMcpServer),
    CallTool {
        server: HttpMcpServer,
        tool: String,
        arguments: serde_json::Value,
    },
}

impl McpRequest {
    #[must_use]
    pub const fn busy(&self) -> McpBusy {
        match self {
            Self::ListTools(_) => McpBusy::ListingTools,
            Self::CallTool { .. } => McpBusy::CallingTool,
        }
    }
}

/// Reply returned from the HTTP MCP worker.
#[derive(Debug, Clone)]
pub enum McpReply {
    Tools(String, Result<Vec<McpTool>, McpRemoteError>),
    Called(Result<McpCallResult, McpRemoteError>),
}

impl McpReply {
    /// Wrap `error` into the reply variant matching `request`.
    #[must_use]
    pub fn fail(request: &McpRequest, error: McpRemoteError) -> Self {
        match request {
            McpRequest::ListTools(server) => Self::Tools(server.name.clone(), Err(error)),
            McpRequest::CallTool { .. } => Self::Called(Err(error)),
        }
    }
}

/// Typed errors for HTTP MCP discovery, header env resolution, TLS, and JSON-RPC.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpRemoteError {
    Disabled,
    Config(String),
    MissingEnv(String),
    Network(String),
    Http(u16, String),
    Protocol(String),
    Worker,
}

impl fmt::Display for McpRemoteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disabled => f.write_str("Unavailable in replay or isolated sessions"),
            Self::MissingEnv(name) => write!(f, "Set {name} environment variable first"),
            Self::Config(msg) | Self::Network(msg) | Self::Protocol(msg) => f.write_str(msg),
            Self::Http(code, msg) => write!(f, "MCP HTTP {code}: {msg}"),
            Self::Worker => f.write_str("MCP worker stopped unexpectedly"),
        }
    }
}

#[cfg(test)]
mod tests;
