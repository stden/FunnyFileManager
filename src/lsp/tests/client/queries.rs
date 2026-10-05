//! Purpose: Queries — cancelled by a document change or a newer query, refused
//! when unsupported, and malformed replies reported to the editor.

use super::*;

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
