//! Purpose: Stub Jira REST transport, JQL recording, and delay gate for BDD.

use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use crate::application::service_jobs::ServiceJobs;
use crate::application::tests::bdd::common::{await_gate, DelayGate};
use crate::jira;
use crate::jira::client::Transport;
use crate::values::jira::{JiraError, JiraReply, JiraRequest};

#[derive(Clone, Copy)]
pub enum Mode {
    Success,
    Unauthorized,
    InvalidJson,
}

pub struct Fixture {
    pub jobs: ServiceJobs,
    pub mode: Mode,
    pub jqls: Arc<Mutex<Vec<String>>>,
    pub gate: DelayGate,
}

impl Default for Fixture {
    fn default() -> Self {
        Self {
            jobs: ServiceJobs::default(),
            mode: Mode::Success,
            jqls: Arc::new(Mutex::new(Vec::new())),
            gate: DelayGate::default(),
        }
    }
}

impl Fixture {
    pub(super) fn failing(kind: &str) -> Self {
        let mode = match kind {
            "unauthorized" => Mode::Unauthorized,
            "invalid json" => Mode::InvalidJson,
            other => panic!("unknown Jira failure {other}"),
        };
        Self {
            mode,
            ..Self::default()
        }
    }

    pub(super) fn delayed() -> Self {
        Self {
            gate: DelayGate::delayed(),
            ..Self::default()
        }
    }

    pub(super) fn work(&mut self) -> impl FnOnce(JiraRequest) -> JiraReply + Send + 'static {
        let mode = self.mode;
        let jqls = Arc::clone(&self.jqls);
        let blocker = self.gate.blocker();
        move |req| {
            await_gate(blocker);
            let mut tr = StubTransport {
                mode,
                jqls: Arc::clone(&jqls),
            };
            jira::execute_with(&mut tr, "https://jira.example.test", req)
        }
    }
}

struct StubTransport {
    mode: Mode,
    jqls: Arc<Mutex<Vec<String>>>,
}

impl Transport for StubTransport {
    fn get(&mut self, path: &str) -> Result<Value, JiraError> {
        if matches!(self.mode, Mode::Unauthorized) {
            return Err(JiraError::Http(401, "invalid token".into()));
        }
        if path.ends_with("/myself") {
            return Ok(json!({"name": "alice", "key": "alice", "displayName": "Alice"}));
        }
        Ok(json!({"key": "DEMO-1", "fields": {"summary": "Fix crash"}}))
    }

    fn post(&mut self, _path: &str, body: &Value) -> Result<Value, JiraError> {
        if let Some(jql) = body.get("jql").and_then(Value::as_str) {
            self.jqls.lock().expect("jqls").push(jql.to_owned());
        }
        match self.mode {
            Mode::Unauthorized => Err(JiraError::Http(401, "invalid token".into())),
            Mode::InvalidJson => Ok(json!(["bad"])),
            Mode::Success => Ok(sample_search_payload()),
        }
    }
}

fn sample_search_payload() -> Value {
    json!({
        "total": 2,
        "issues": [
            {
                "key": "DEMO-1",
                "fields": {
                    "summary": "Fix crash",
                    "description": "Repro steps here",
                    "status": {"name": "Open"},
                    "assignee": {"displayName": "Alice"},
                    "priority": {"name": "High"},
                    "issuetype": {"name": "Bug"}
                }
            },
            {
                "key": "DEMO-2",
                "fields": {
                    "summary": "Add export",
                    "description": "Markdown summary",
                    "status": {"name": "In Progress"}
                }
            }
        ]
    })
}
