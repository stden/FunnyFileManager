//! Purpose: Initialization ordering, capability negotiation, and saves. The
//! server's own traffic is pinned in `client/incoming.rs`, queries and stale
//! document results in `client/queries.rs`.

use super::super::{
    client::{Client, Event},
    positions::Position,
    types::*,
};
use super::documents::snapshot;
use serde_json::{json, Value};

mod incoming;
mod queries;

fn initialized(sync: Value) -> Client {
    let mut client = Client::new("rust".into(), "file:///workspace".into());
    client.receive(json!({"jsonrpc":"2.0","id":1,"result":{"capabilities":{
        "textDocumentSync":sync,"completionProvider":{},"hoverProvider":true,"definitionProvider":true
    }}})).unwrap();
    client
}

fn open(client: &mut Client) -> Snapshot {
    let temp = tempfile::TempDir::new().unwrap();
    let mut document = snapshot("fn main() {}\n");
    document.path = temp.path().join("main.rs");
    client.synchronize(document.clone()).unwrap();
    document
}

#[test]
fn initialize_advertises_utf16_and_precedes_all_document_messages() {
    let mut client = Client::new("rust".into(), "file:///workspace".into());
    assert!(!client.is_ready());
    assert!(client.synchronize(snapshot("early")).is_err());
    let request = &client.rpc.outbox[0];
    assert_eq!(request["method"], "initialize");
    assert_eq!(
        request["params"]["capabilities"]["general"]["positionEncodings"],
        json!(["utf-16"])
    );
    assert_eq!(
        request["params"]["capabilities"]["workspace"]["didChangeConfiguration"]
            ["dynamicRegistration"],
        true
    );
    assert_eq!(
        request["params"]["capabilities"]["textDocument"]["completion"]["completionItem"]
            ["snippetSupport"],
        false
    );
    client
        .receive(json!({"jsonrpc":"2.0","id":1,"result":{"capabilities":{"textDocumentSync":2}}}))
        .unwrap();
    assert!(client.is_ready());
    assert_eq!(client.rpc.outbox[1]["method"], "initialized");
    open(&mut client);
    assert_eq!(client.rpc.outbox[2]["method"], "textDocument/didOpen");
}

#[test]
fn unsupported_encoding_sync_and_initialize_errors_fail_cleanly() {
    for result in [
        json!({"positionEncoding":"utf-8"}),
        json!({"textDocumentSync":3}),
    ] {
        let mut client = Client::new("rust".into(), "file:///workspace".into());
        assert!(client
            .receive(json!({"jsonrpc":"2.0","id":1,"result":{"capabilities":result}}))
            .is_err());
        assert!(!client.is_ready());
    }
    let mut client = Client::new("rust".into(), "file:///workspace".into());
    assert!(client
        .receive(json!({"jsonrpc":"2.0","id":1,"error":{"code":-1,"message":"bad"}}))
        .is_err());
}

#[test]
fn saves_follow_negotiated_text_options_and_shutdown_closes_documents_first() {
    let mut client = initialized(json!({"openClose":true,"change":2,"save":{"includeText":true}}));
    let mut document = open(&mut client);
    document.saved += 1;
    client.synchronize(document.clone()).unwrap();
    assert_eq!(
        client.rpc.outbox.last().unwrap()["method"],
        "textDocument/didSave"
    );
    assert_eq!(
        client.rpc.outbox.last().unwrap()["params"]["text"],
        document.text
    );
    client.shutdown();
    let messages = &client.rpc.outbox;
    assert_eq!(
        messages[messages.len() - 2]["method"],
        "textDocument/didClose"
    );
    assert_eq!(messages.last().unwrap()["method"], "shutdown");
    let id = messages.last().unwrap()["id"].clone();
    client
        .receive(json!({"jsonrpc":"2.0","id":id,"result":null}))
        .unwrap();
    assert!(client.shutdown_complete);
}

#[test]
fn saves_without_text_include_only_the_document_uri() {
    let mut client = initialized(json!({"openClose":true,"change":2,"save":true}));
    let mut document = open(&mut client);
    document.saved += 1;
    client.synchronize(document).unwrap();
    let message = client.rpc.outbox.last().unwrap();
    assert_eq!(message["method"], "textDocument/didSave");
    assert_eq!(message["params"].as_object().unwrap().len(), 1);
    assert!(message["params"]["textDocument"]["uri"].is_string());
}
