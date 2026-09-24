//! Purpose: What the client does with a server's unprompted traffic — query
//! replies, published diagnostics and the server's own requests.

use serde_json::{json, Value};

use super::{Client, Event};
use crate::lsp::{
    features::{self, Diagnostic},
    rpc::ResponseError,
    types::Query,
    uri,
};

impl Client {
    pub(super) fn answer(&mut self, query: Query, result: Result<Value, ResponseError>) {
        if !self
            .document
            .as_ref()
            .is_some_and(|doc| doc.snapshot.stamp == query.stamp)
        {
            return;
        }
        let result = result.and_then(|value| {
            features::parse(query.feature, &value).map_err(|message| ResponseError {
                code: -32602,
                message: message.into(),
                data: Value::Null,
            })
        });
        if let Err(error) = &result {
            crate::diagnostics::log_error(&format!("LSP response error {}", error.code));
        }
        self.events.push(Event::Reply(query, result));
    }

    pub(super) fn notification(
        &mut self,
        method: &str,
        params: &Value,
    ) -> Result<(), &'static str> {
        if method != "textDocument/publishDiagnostics" {
            return Ok(());
        }
        let Some(doc) = self.document.as_ref().filter(|doc| {
            params["uri"]
                .as_str()
                .is_some_and(|value| uri::same(value, &doc.uri))
        }) else {
            return Ok(());
        };
        if params["version"]
            .as_i64()
            .is_some_and(|version| version != i64::from(doc.version))
        {
            return Ok(());
        }
        let mut diagnostics: Vec<Diagnostic> =
            serde_json::from_value(params["diagnostics"].clone())
                .map_err(crate::lsp::logged("Invalid LSP diagnostics"))?;
        diagnostics.truncate(200);
        self.events
            .push(Event::Diagnostics(doc.snapshot.stamp, diagnostics));
        Ok(())
    }

    pub(super) fn server_request(&mut self, id: &Value, method: &str, params: &Value) {
        let result = match method {
            "workspace/configuration" => json!(vec![
                Value::Null;
                params["items"].as_array().map_or(0, Vec::len)
            ]),
            "workspace/workspaceFolders" => json!([{"uri":self.root_uri,"name":"workspace"}]),
            "window/workDoneProgress/create" => Value::Null,
            _ => {
                self.rpc.outbox.push(json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":"Unsupported client request"}}));
                return;
            }
        };
        self.rpc
            .outbox
            .push(json!({"jsonrpc":"2.0","id":id,"result":result}));
    }
}
