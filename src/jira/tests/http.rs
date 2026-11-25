//! Purpose: Unit tests for Jira HTTP transport over loopback TCP and error formatting.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use serde_json::json;

use super::super::*;
use crate::values::jira::JiraUser;

fn spawn_one_shot(status: &str, body: &str) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    let reply = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let handle = thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0u8; 2048];
            drop(stream.read(&mut buf));
            drop(stream.write_all(reply.as_bytes()));
        }
    });
    (format!("http://{addr}"), handle)
}

#[test]
fn http_transport_reads_json_handles_204_and_reports_http_errors() {
    let token = config::Token::try_new_for_test("tok");
    let (url, h1) = spawn_one_shot("200 OK", r#"{"key":"DEMO-1","fields":{"summary":"Hi"}}"#);
    let mut client = HttpTransport::with_base(token.clone(), url).expect("client");
    let got = client.get("/rest/api/2/issue/DEMO-1").expect("get");
    assert_eq!(got.get("key").and_then(|v| v.as_str()), Some("DEMO-1"));
    drop(h1.join());

    let (url2, h2) = spawn_one_shot("204 No Content", "");
    let mut client2 = HttpTransport::with_base(token.clone(), url2).expect("client2");
    assert_eq!(
        client2.post("/rest/api/2/search", &json!({})).expect("204"),
        json!(null)
    );
    drop(h2.join());

    let (url3, h3) = spawn_one_shot("401 Unauthorized", "bad token");
    let mut client3 = HttpTransport::with_base(token, url3).expect("client3");
    let err = client3.get("/rest/api/2/myself").unwrap_err();
    assert!(matches!(err, JiraError::Http(401, _)));
    assert!(err.to_string().contains("401"));
    drop(h3.join());
}

#[test]
fn http_transport_executes_export_key_and_snapshot_over_stub() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut tr = super::StubTransport::new(vec![
        Ok(json!({"name": "alice", "key": "alice"})),
        Ok(json!({"key": "DEMO-9", "fields": {"summary": "Single"}})),
        Ok(json!({"name": "alice"})),
        Ok(json!({"total": 1, "issues": [{"key": "DEMO-9", "fields": {"summary": "Single"}}]})),
    ]);
    let r1 = execute_with(
        &mut tr,
        "https://jira.example.test",
        JiraRequest::ExportKey {
            key: "DEMO-9".into(),
            output_dir: temp.path().to_path_buf(),
            force: false,
        },
    );
    assert!(
        matches!(r1, JiraReply::Exported(Ok(_))) && temp.path().join("DEMO-9/issue.md").is_file()
    );
    let r2 = execute_with(
        &mut tr,
        "https://jira.example.test",
        JiraRequest::ExportSnapshot {
            jql: "assignee = currentUser()".into(),
            page_size: 50,
            output_dir: temp.path().to_path_buf(),
        },
    );
    assert!(matches!(r2, JiraReply::Exported(Ok(_))) && temp.path().join("jira_exports").is_dir());
}

#[test]
fn execute_exports_cached_tickets_without_network() {
    let temp = tempfile::TempDir::new().expect("temp");
    let cfg = RuntimeConfig {
        base_url: "https://jira.example.test".into(),
        token: config::Token::try_new_for_test("t"),
        tls_verify: true,
        ca_cert: None,
        page_size: 50,
    };
    let r3 = execute(
        &cfg,
        JiraRequest::ExportTickets {
            issues: Vec::new(),
            user: Some(JiraUser {
                name: "alice".into(),
                ..JiraUser::default()
            }),
            base_url: cfg.base_url.clone(),
            output_dir: temp.path().to_path_buf(),
            force: false,
        },
    );
    assert!(matches!(r3, JiraReply::Exported(Ok(_))));
}

#[test]
fn execute_with_exports_cached_tickets_directly() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut stub = super::StubTransport::new(Vec::new());
    let r4 = execute_with(
        &mut stub,
        "https://jira.example.test",
        JiraRequest::ExportTickets {
            issues: Vec::new(),
            user: Some(JiraUser::default()),
            base_url: "https://jira.example.test".into(),
            output_dir: temp.path().to_path_buf(),
            force: false,
        },
    );
    assert!(matches!(r4, JiraReply::Exported(Ok(_))));
}
