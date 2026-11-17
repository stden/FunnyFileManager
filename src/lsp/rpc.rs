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
pub struct ResponseError {
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
        let message = json!({"jsonrpc":"2.0", "id":id, "method":method});
        self.outbox.push(with_params(message, params));
        crate::diagnostics::log_action(&format!("LSP request {} {method}", id.0));
        id
    }

    pub fn notify(&mut self, method: &str, params: Value) {
        let message = json!({"jsonrpc":"2.0", "method":method});
        self.outbox.push(with_params(message, params));
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

    pub fn receive(&mut self, mut message: Value) -> Result<Incoming, &'static str> {
        if message.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
            return Err("Invalid JSON-RPC version");
        }
        if let Some(method) = message.get("method") {
            let method = method.as_str().ok_or("Invalid JSON-RPC method")?.to_owned();
            // Absent `params` are `Null`, as JSON-RPC treats them.
            let params = message
                .get_mut("params")
                .map(Value::take)
                .unwrap_or_default();
            return Ok(match message.get("id") {
                Some(id) => Incoming::Request(id.clone(), method, params),
                None => Incoming::Notification(method, params),
            });
        }
        let result = response(&message)?;
        let pending = message
            .get("id")
            .and_then(Value::as_u64)
            .and_then(|id| self.pending.remove(&RequestId(id)));
        Ok(pending.map_or(Incoming::Unknown, |pending| {
            Incoming::Response(pending, result)
        }))
    }
}

/// `message`, a `json!` object, with `params` moved in rather than copied.
fn with_params(mut message: Value, params: Value) -> Value {
    if let Some(fields) = message.as_object_mut() {
        fields.insert("params".to_owned(), params);
    }
    message
}

fn response(message: &Value) -> Result<Result<Value, ResponseError>, &'static str> {
    if !message
        .get("id")
        .is_some_and(|id| id.is_number() || id.is_string())
    {
        return Err("Invalid JSON-RPC response ID");
    }
    match (message.get("result"), message.get("error")) {
        (Some(result), None) => Ok(Ok(result.clone())),
        (None, Some(error)) => serde_json::from_value(error.clone())
            .map(Err)
            .map_err(crate::lsp::logged("Invalid JSON-RPC error")),
        _ => Err("Invalid JSON-RPC response"),
    }
}
