//! Purpose: Stub Streamable HTTP JSON-RPC transport and `.mcp.json` fixture helpers.

use std::path::Path;
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use crate::application::service_jobs::ServiceJobs;
use crate::application::tests::bdd::common::{await_gate, DelayGate};
use crate::mcp_remote::client::{RpcResponse, RpcTransport};
use crate::mcp_remote::execute_with;
use crate::values::mcp_remote::{McpRemoteError, McpReply, McpRequest};

#[derive(Clone, Copy)]
pub enum Mode {
    Success,
    Http403,
}

pub struct Fixture {
    pub jobs: ServiceJobs,
    pub mode: Mode,
    pub calls: Arc<Mutex<Vec<String>>>,
    pub gate: DelayGate,
}

impl Default for Fixture {
    fn default() -> Self {
        Self {
            jobs: ServiceJobs::default(),
            mode: Mode::Success,
            calls: Arc::new(Mutex::new(Vec::new())),
            gate: DelayGate::default(),
        }
    }
}

impl Fixture {
    pub(super) fn http_error() -> Self {
        Self {
            mode: Mode::Http403,
            ..Self::default()
        }
    }

    pub(super) fn delayed() -> Self {
        Self {
            gate: DelayGate::delayed(),
            ..Self::default()
        }
    }

    pub(super) fn work(&mut self) -> impl FnOnce(McpRequest) -> McpReply + Send + 'static {
        let mode = self.mode;
        let calls = Arc::clone(&self.calls);
        let blocker = self.gate.blocker();
        move |req| {
            await_gate(blocker);
            let mut tr = StubRpcTransport { mode, calls };
            execute_with(&mut tr, req)
        }
    }
}

pub(super) fn write_project_mcp(panel_dir: &Path, name: &str, disabled: &str) {
    let payload = json!({
        "mcpServers": {
            name: {
                "type": "http",
                "url": "https://mcp.example.test/rpc",
                "disabledTools": [disabled]
            }
        }
    });
    std::fs::write(panel_dir.join(".mcp.json"), payload.to_string()).expect("write .mcp.json");
}

struct StubRpcTransport {
    mode: Mode,
    calls: Arc<Mutex<Vec<String>>>,
}

impl RpcTransport for StubRpcTransport {
    fn post(
        &mut self,
        _session_id: Option<&str>,
        body: &Value,
    ) -> Result<RpcResponse, McpRemoteError> {
        let method = body
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        self.calls.lock().expect("calls").push(method.to_owned());
        if matches!(self.mode, Mode::Http403) {
            return Err(McpRemoteError::Http(403, "forbidden".into()));
        }
        Ok(RpcResponse {
            content_type: "application/json".into(),
            session_id: Some("sid-1".into()),
            body: rpc_payload(method),
        })
    }
}

fn rpc_payload(method: &str) -> String {
    match method {
        "initialize" => {
            json!({"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-03-26"}}).to_string()
        }
        "tools/list" => tools_list_payload().to_string(),
        "tools/call" => json!({
            "jsonrpc": "2.0",
            "id": 2,
            "result": {"content": [{"type": "text", "text": "matched 3 files"}]}
        })
        .to_string(),
        _ => String::new(),
    }
}

fn tools_list_payload() -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": 2,
        "result": {
            "tools": [
                {
                    "name": "search",
                    "description": "Search docs",
                    "inputSchema": {
                        "type": "object",
                        "required": ["query"],
                        "properties": {"query": {"type": "string"}}
                    }
                },
                {"name": "hidden_tool", "description": "Hidden"}
            ]
        }
    })
}
