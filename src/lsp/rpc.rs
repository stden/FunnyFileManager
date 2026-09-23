//! Purpose: JSON-RPC request IDs, cancellation, and lossless server error responses.

use super::types::Query;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub(super) struct RequestId(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Pending {
    Initialize,
    Query(Query),
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct ResponseError {
    pub code: i64,
    pub message: String,
    #[serde(default)]
    pub data: Value,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Incoming {
    Response(Pending, Result<Value, ResponseError>),
    Notification(String, Value),
    Request(Value, String, Value),
    Unknown,
}

#[derive(Default)]
pub(super) struct Session {
    next: u64,
    pending: BTreeMap<RequestId, Pending>,
    pub outbox: Vec<Value>,
}

impl Session {
    pub fn request(&mut self, method: &str, params: Value, context: Pending) -> RequestId {
        self.next += 1;
        let id = RequestId(self.next);
        self.pending.insert(id, context);
        self.outbox
            .push(json!({"jsonrpc":"2.0", "id":id, "method":method, "params":params}));
        crate::diagnostics::log_action(&format!("LSP request {} {method}", id.0));
        id
    }

    pub fn notify(&mut self, method: &str, params: Value) {
        self.outbox
            .push(json!({"jsonrpc":"2.0", "method":method, "params":params}));
    }

    pub fn cancel_queries(&mut self) {
        let ids: Vec<_> = self
            .pending
            .iter()
            .filter_map(|(id, pending)| matches!(pending, Pending::Query(_)).then_some(*id))
            .collect();
        for id in ids {
            self.pending.remove(&id);
            self.notify("$/cancelRequest", json!({"id":id}));
        }
    }

    pub fn receive(&mut self, message: Value) -> Result<Incoming, &'static str> {
        if message["jsonrpc"] != "2.0" {
            return Err("Invalid JSON-RPC version");
        }
        if let Some(method) = message.get("method") {
            let method = method.as_str().ok_or("Invalid JSON-RPC method")?.to_owned();
            return Ok(message.get("id").map_or_else(
                || Incoming::Notification(method.clone(), message["params"].clone()),
                |id| Incoming::Request(id.clone(), method.clone(), message["params"].clone()),
            ));
        }
        let result = response(&message)?;
        let pending = message["id"]
            .as_u64()
            .and_then(|id| self.pending.remove(&RequestId(id)));
        Ok(pending.map_or(Incoming::Unknown, |pending| {
            Incoming::Response(pending, result)
        }))
    }
}

fn response(message: &Value) -> Result<Result<Value, ResponseError>, &'static str> {
    match (message.get("result"), message.get("error")) {
        (Some(result), None) => Ok(Ok(result.clone())),
        (None, Some(error)) => serde_json::from_value(error.clone())
            .map(Err)
            .map_err(|_| "Invalid JSON-RPC error"),
        _ => Err("Invalid JSON-RPC response"),
    }
}
