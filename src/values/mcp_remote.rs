//! Purpose: Plain values shared between the Streamable HTTP MCP client, UI state, and ops.

mod request;
mod types;

pub use request::{McpBusy, McpRemoteError, McpReply, McpRequest};
pub use types::{HttpMcpServer, McpAccess, McpCallResult, McpTool};
