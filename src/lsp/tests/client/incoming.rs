//! Purpose: What the server sends unprompted — diagnostics for the current
//! document and version only, and requests answered or refused.

use super::*;

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
fn configuration_change_registrations_and_unregistrations_are_acknowledged() {
    let mut client = initialized(json!(2));
    for (method, key) in [
        ("client/registerCapability", "registrations"),
        ("client/unregisterCapability", "unregisterations"),
    ] {
        client
            .receive(
                json!({"jsonrpc":"2.0", "id":"configuration", "method":method,
                "params":{key:[{"id":"settings", "method":"workspace/didChangeConfiguration"}]}}),
            )
            .unwrap();
        let reply = client.rpc.outbox.last().unwrap();
        assert_eq!(reply["id"], "configuration");
        assert_eq!(reply.get("result"), Some(&Value::Null));
        assert!(reply.get("error").is_none());
    }
}

#[test]
fn unimplemented_and_malformed_registrations_are_rejected() {
    let mut client = initialized(json!(2));
    for registrations in [
        json!([{"id":"watch", "method":"workspace/didChangeWatchedFiles"}]),
        json!([{"id":"settings", "method":"workspace/didChangeConfiguration"},
            {"id":"watch", "method":"workspace/didChangeWatchedFiles"}]),
        json!([{"method":"workspace/didChangeConfiguration"}]),
        json!({"id":"settings", "method":"workspace/didChangeConfiguration"}),
        Value::Null,
    ] {
        client
            .receive(
                json!({"jsonrpc":"2.0", "id":40, "method":"client/registerCapability",
                "params":{"registrations":registrations}}),
            )
            .unwrap();
        let reply = client.rpc.outbox.last().unwrap();
        assert_eq!(reply["id"], 40);
        assert_eq!(reply["error"]["code"], -32601);
        assert!(reply.get("result").is_none());
    }
}
