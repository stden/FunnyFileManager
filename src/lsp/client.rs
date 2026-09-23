//! Purpose: Pure LSP initialization, capability negotiation, and document/query session.

use super::{
    document::Document,
    features::{self, Diagnostic, Reply},
    rpc::{Incoming, Pending, ResponseError, Session},
    types::{Query, Snapshot, Stamp},
    uri,
};
use serde_json::{json, Value};

mod capabilities;
use capabilities::Capabilities;

#[derive(Debug, Clone)]
pub enum Event {
    TrustRequired(std::path::PathBuf),
    State(super::types::ServerState),
    Diagnostics(Stamp, Vec<Diagnostic>),
    Reply(Query, Result<Reply, ResponseError>),
}

pub(super) struct Client {
    pub rpc: Session,
    pub document: Option<Document>,
    capabilities: Option<Capabilities>,
    pub events: Vec<Event>,
    language: String,
    root_uri: String,
    pub shutdown_complete: bool,
}

impl Client {
    pub fn new(language: String, root_uri: String) -> Self {
        let mut rpc = Session::default();
        rpc.request(
            "initialize",
            capabilities::initialize(&root_uri),
            Pending::Initialize,
        );
        Self {
            rpc,
            document: None,
            capabilities: None,
            events: Vec::new(),
            language,
            root_uri,
            shutdown_complete: false,
        }
    }

    pub const fn is_ready(&self) -> bool {
        self.capabilities.is_some()
    }

    pub fn receive(&mut self, message: Value) -> Result<(), &'static str> {
        match self.rpc.receive(message)? {
            Incoming::Response(Pending::Initialize, result) => self.initialized(result)?,
            Incoming::Response(Pending::Query(query), result) => self.answer(query, result),
            Incoming::Response(Pending::Shutdown, _) => self.shutdown_complete = true,
            Incoming::Notification(method, params) => self.notification(&method, &params)?,
            Incoming::Request(id, method, params) => self.server_request(&id, &method, &params),
            Incoming::Unknown => {}
        }
        Ok(())
    }

    fn initialized(&mut self, result: Result<Value, ResponseError>) -> Result<(), &'static str> {
        let value = result.map_err(|error| {
            crate::diagnostics::log_error(&format!("LSP initialize error {}", error.code));
            "LSP initialization rejected"
        })?;
        self.capabilities = Some(Capabilities::parse(&value["capabilities"])?);
        self.rpc.notify("initialized", json!({}));
        self.events
            .push(Event::State(super::types::ServerState::Ready));
        crate::diagnostics::log_action("LSP initialized");
        Ok(())
    }

    pub fn synchronize(&mut self, snapshot: Snapshot) -> Result<(), &'static str> {
        if !self.is_ready() {
            return Err("LSP is not ready");
        }
        let uri = uri::from_path(&snapshot.path).ok_or("Invalid document path")?;
        if self
            .document
            .as_ref()
            .is_some_and(|doc| doc.snapshot.stamp.id != snapshot.stamp.id)
        {
            self.close();
        }
        let caps = self.capabilities.as_ref().ok_or("LSP is not ready")?;
        self.rpc.cancel_queries();
        if let Some(document) = &mut self.document {
            let saved = document.snapshot.saved != snapshot.saved;
            document.update(snapshot, caps.change, &mut self.rpc)?;
            caps.save(document, saved, &mut self.rpc);
        } else {
            self.document = Some(if caps.open_close {
                Document::open(snapshot, uri, &self.language, &mut self.rpc)
            } else {
                Document {
                    snapshot,
                    uri,
                    version: 1,
                }
            });
        }
        Ok(())
    }

    pub fn close(&mut self) {
        self.rpc.cancel_queries();
        if let Some(document) = self.document.take() {
            if self
                .capabilities
                .as_ref()
                .is_some_and(|caps| caps.open_close)
            {
                document.close(&mut self.rpc);
            }
        }
    }

    pub fn query(&mut self, query: Query) -> Result<(), &'static str> {
        self.rpc.cancel_queries();
        let document = self
            .document
            .as_ref()
            .filter(|doc| doc.snapshot.stamp == query.stamp)
            .ok_or("Document changed")?;
        if !self
            .capabilities
            .as_ref()
            .is_some_and(|caps| caps.supports(query.feature))
        {
            return Err("Language server does not support this feature");
        }
        self.rpc.request(
            query.feature.method(),
            json!({"textDocument":{"uri":document.uri},"position":query.position}),
            Pending::Query(query),
        );
        Ok(())
    }

    fn answer(&mut self, query: Query, result: Result<Value, ResponseError>) {
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

    fn notification(&mut self, method: &str, params: &Value) -> Result<(), &'static str> {
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
                .map_err(|_| "Invalid LSP diagnostics")?;
        diagnostics.truncate(200);
        self.events
            .push(Event::Diagnostics(doc.snapshot.stamp, diagnostics));
        Ok(())
    }

    fn server_request(&mut self, id: &Value, method: &str, params: &Value) {
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

    pub fn shutdown(&mut self) {
        self.close();
        self.rpc.request("shutdown", Value::Null, Pending::Shutdown);
    }
}
