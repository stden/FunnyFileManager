//! Purpose: Initialization ordering, capability negotiation, and stale document results.

use super::super::{
    client::{Client, Event},
    positions::Position,
    types::*,
};
use super::documents::snapshot;
use serde_json::{json, Value};

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
fn diagnostics_require_the_current_uri_and_version() {
    let mut client = initialized(json!(2));
    open(&mut client);
    let uri = client.document.as_ref().unwrap().uri.clone();
    let diagnostic = json!({"range":{"start":{"line":0,"character":3},"end":{"line":0,"character":7}},"severity":1,"message":"bad"});
    for (uri, version) in [
        ("file:///wrong.rs", 1),
        (uri.as_str(), 0),
        (uri.as_str(), 2),
    ] {
        client.receive(json!({"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":uri,"version":version,"diagnostics":[diagnostic]}})).unwrap();
    }
    assert_eq!(client.events.len(), 1);
    client.receive(json!({"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":uri,"version":1,"diagnostics":[diagnostic]}})).unwrap();
    let Event::Diagnostics(_, items) = client.events.last().unwrap() else {
        panic!("diagnostics")
    };
    assert_eq!(items[0].message, "bad");
    assert_eq!(items[0].range.start.character, 3);
}

#[cfg(windows)]
#[test]
fn diagnostics_accept_equivalent_windows_drive_spelling() {
    let mut client = initialized(json!(2));
    open(&mut client);
    let uri = client.document.as_ref().unwrap().uri.clone();
    let lower = uri.to_ascii_lowercase();
    let upper = uri.to_ascii_uppercase();
    assert_ne!(lower, upper);
    client.document.as_mut().unwrap().uri = upper;
    client.receive(json!({"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":lower,"version":1,"diagnostics":[]}})).unwrap();
    assert!(matches!(
        client.events.last(),
        Some(Event::Diagnostics(_, _))
    ));
}

#[test]
fn changing_a_document_cancels_queries_and_late_responses_do_nothing() {
    let mut client = initialized(json!(1));
    let mut document = open(&mut client);
    let query = Query {
        stamp: document.stamp,
        position: Position::default(),
        feature: Feature::Completion,
    };
    client.query(query).unwrap();
    let id = client.rpc.outbox.last().unwrap()["id"].clone();
    document.text.push('x');
    document.stamp.revision.0 += 1;
    client.synchronize(document).unwrap();
    let events = client.events.len();
    client
        .receive(json!({"jsonrpc":"2.0","id":id,"result":[{"label":"stale"}]}))
        .unwrap();
    assert_eq!(client.events.len(), events);
    assert!(client.query(query).is_err());
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
#[test]
fn server_requests_receive_supported_responses_and_unknown_methods_receive_errors() {
    let mut client = initialized(json!({"openClose":false,"change":0}));
    for (id, method, params, result) in [
        (
            10,
            "workspace/configuration",
            json!({"items":[{},{}]}),
            json!([null, null]),
        ),
        (
            11,
            "workspace/workspaceFolders",
            json!({}),
            json!([{"uri":"file:///workspace","name":"workspace"}]),
        ),
        (12, "window/workDoneProgress/create", json!({}), Value::Null),
    ] {
        client
            .receive(json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))
            .unwrap();
        assert_eq!(client.rpc.outbox.last().unwrap()["result"], result);
    }
}

#[test]
fn unsupported_server_requests_and_disabled_sync_are_honored() {
    let mut client = initialized(json!({"openClose":false,"change":0}));
    client
        .receive(json!({"jsonrpc":"2.0","id":"request","method":"workspace/applyEdit"}))
        .unwrap();
    assert_eq!(client.rpc.outbox.last().unwrap()["error"]["code"], -32601);
    client
        .receive(json!({"jsonrpc":"2.0","method":"window/logMessage","params":{}}))
        .unwrap();
    open(&mut client);
    assert!(!client
        .rpc
        .outbox
        .iter()
        .any(|message| message["method"] == "textDocument/didOpen"));
    client.close();
}

#[test]
fn unsupported_features_and_malformed_diagnostics_fail_without_changing_documents() {
    let mut client = Client::new("rust".into(), "file:///workspace".into());
    client
        .receive(json!({"jsonrpc":"2.0","id":1,"result":{"capabilities":{"textDocumentSync":1}}}))
        .unwrap();
    let document = open(&mut client);
    let query = Query {
        stamp: document.stamp,
        position: Position::default(),
        feature: Feature::Hover,
    };
    assert!(client.query(query).is_err());
    let uri = client.document.as_ref().unwrap().uri.clone();
    assert!(client.receive(json!({"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":uri,"diagnostics":"bad"}})).is_err());
    assert_eq!(
        client.document.as_ref().unwrap().snapshot.stamp,
        document.stamp
    );
}
#[test]
fn malformed_feature_payloads_and_jsonrpc_errors_reach_the_editor() {
    let mut client = initialized(json!(2));
    let document = open(&mut client);
    let query = Query {
        stamp: document.stamp,
        position: Position::default(),
        feature: Feature::Completion,
    };
    for payload in [
        json!({"result":42}),
        json!({"error":{"code":-32000,"message":"failure","data":{"reason":1}}}),
    ] {
        client.query(query).unwrap();
        let id = client.rpc.outbox.last().unwrap()["id"].clone();
        let mut response = payload;
        response["jsonrpc"] = json!("2.0");
        response["id"] = id;
        client.receive(response).unwrap();
        assert!(matches!(
            client.events.last(),
            Some(Event::Reply(_, Err(_)))
        ));
    }
    let next = open(&mut client);
    assert_ne!(document.stamp.id, next.stamp.id);
    assert!(client
        .rpc
        .outbox
        .iter()
        .any(|message| message["method"] == "textDocument/didClose"));
}

#[test]
fn an_unsupported_query_cancels_the_previous_supported_request() {
    let mut client = Client::new("rust".into(), "file:///workspace".into());
    client
        .receive(json!({"jsonrpc":"2.0","id":1,"result":{"capabilities":{
            "textDocumentSync":2,"completionProvider":{}
        }}}))
        .unwrap();
    let document = open(&mut client);
    let mut query = Query {
        stamp: document.stamp,
        position: Position::default(),
        feature: Feature::Completion,
    };
    client.query(query).unwrap();
    let id = client.rpc.outbox.last().unwrap()["id"].clone();
    query.feature = Feature::Hover;
    assert!(client.query(query).is_err());
    assert_eq!(
        client.rpc.outbox.last().unwrap()["method"],
        "$/cancelRequest"
    );
    let events = client.events.len();
    client
        .receive(json!({"jsonrpc":"2.0","id":id,"result":[{"label":"late"}]}))
        .unwrap();
    assert_eq!(client.events.len(), events);
}
