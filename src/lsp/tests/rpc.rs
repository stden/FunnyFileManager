//! Purpose: Request matching, lossless errors, notifications, and cancellation.

use super::super::{positions::Position, rpc::*, types::*};
use serde_json::json;

fn query() -> Query {
    Query {
        stamp: Stamp {
            id: DocumentId::default(),
            revision: Revision::default(),
        },
        position: Position::default(),
        feature: Feature::Completion,
    }
}

#[test]
fn responses_match_ids_even_when_the_server_answers_out_of_order() {
    let mut rpc = Session::default();
    let first = rpc.request("initialize", json!({}), Pending::Initialize);
    let second = rpc.request("shutdown", json!(null), Pending::Shutdown);
    assert_eq!(
        rpc.receive(json!({"jsonrpc":"2.0","id":second,"result":null}))
            .unwrap(),
        Incoming::Response(Pending::Shutdown, Ok(json!(null)))
    );
    assert_eq!(
        rpc.receive(json!({"jsonrpc":"2.0","id":first,"result":{}}))
            .unwrap(),
        Incoming::Response(Pending::Initialize, Ok(json!({})))
    );
    assert_eq!(
        rpc.receive(json!({"jsonrpc":"2.0","id":999,"result":null}))
            .unwrap(),
        Incoming::Unknown
    );
}

#[test]
fn server_errors_preserve_code_message_and_data() {
    let mut rpc = Session::default();
    let id = rpc.request("initialize", json!({}), Pending::Initialize);
    let error = json!({"code":-32002,"message":"not ready","data":{"retry":false}});
    let Incoming::Response(Pending::Initialize, Err(result)) = rpc
        .receive(json!({"jsonrpc":"2.0","id":id,"error":error}))
        .unwrap()
    else {
        panic!("error lost")
    };
    assert_eq!(result.code, -32002);
    assert_eq!(result.message, "not ready");
    assert_eq!(result.data, json!({"retry":false}));
}

#[test]
fn notifications_and_server_requests_are_not_client_responses() {
    let mut rpc = Session::default();
    rpc.notify("initialized", json!({}));
    assert!(rpc.outbox[0].get("id").is_none());
    assert_eq!(
        rpc.receive(json!({"jsonrpc":"2.0","method":"notice","params":{}}))
            .unwrap(),
        Incoming::Notification("notice".into(), json!({}))
    );
    assert_eq!(
        rpc.receive(json!({"jsonrpc":"2.0","id":"server-1","method":"config","params":[]}))
            .unwrap(),
        Incoming::Request(json!("server-1"), "config".into(), json!([]))
    );
}

#[test]
fn cancellation_removes_pending_queries_and_ignores_late_results() {
    let mut rpc = Session::default();
    let query = query();
    let id = rpc.request(query.feature.method(), json!({}), Pending::Query(query));
    rpc.cancel_queries();
    assert_eq!(rpc.outbox.last().unwrap()["method"], "$/cancelRequest");
    assert_eq!(rpc.outbox.last().unwrap()["params"]["id"], json!(id));
    assert_eq!(
        rpc.receive(json!({"jsonrpc":"2.0","id":id,"result":[]}))
            .unwrap(),
        Incoming::Unknown
    );
}

#[test]
fn malformed_rpc_envelopes_do_not_masquerade_as_success() {
    let mut rpc = Session::default();
    for message in [
        json!({}),
        json!({"jsonrpc":"1.0"}),
        json!({"jsonrpc":"2.0","method":9}),
        json!({"jsonrpc":"2.0","id":1}),
        json!({"jsonrpc":"2.0","id":1,"error":{}}),
        json!({"jsonrpc":"2.0","id":1,"result":null,"error":{}}),
    ] {
        assert!(rpc.receive(message).is_err());
    }
}
#[test]
fn response_without_an_id_is_rejected() {
    let mut session = Session::default();
    assert!(session
        .receive(json!({"jsonrpc":"2.0","result":null}))
        .is_err());
}
