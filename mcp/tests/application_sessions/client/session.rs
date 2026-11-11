//! Purpose: Drive and settle real asynchronous application actions through MCP tools.

use super::*;
use std::time::Instant;

impl Client {
    pub(in super::super) async fn start(&mut self, path: &Path) -> SessionId {
        let started = self
            .tool(
                "ffm_session_start",
                json!({"path":path.to_string_lossy(),
            "profile":"isolated","width":80,"height":24}),
            )
            .await;
        assert_eq!(started["protocol_version"], 1);
        assert_eq!(started["state"]["screen"]["width"], 80);
        SessionId(
            started["session_id"]
                .as_str()
                .expect("opaque session ID")
                .to_owned(),
        )
    }

    pub(in super::super) async fn session(
        &mut self,
        name: &str,
        id: &SessionId,
        mut arguments: Value,
    ) -> Value {
        arguments
            .as_object_mut()
            .expect("tool arguments")
            .insert("session_id".to_owned(), json!(id.0));
        self.tool(name, arguments).await
    }

    pub(in super::super) async fn state(&mut self, id: &SessionId) -> Value {
        self.session("ffm_session_state", id, json!({})).await["state"].clone()
    }

    pub(in super::super) async fn settle(&mut self, id: &SessionId, mut state: Value) -> Value {
        let deadline = Instant::now() + Duration::from_secs(10);
        while state["busy"] == true || state["background_pending"] == true {
            assert!(
                Instant::now() < deadline,
                "application job did not finish: {state}"
            );
            state = self
                .session("ffm_session_wait", id, json!({"timeout_ms":100}))
                .await["state"]
                .clone();
        }
        assert_eq!(state["busy"], false);
        state
    }

    pub(in super::super) async fn perform(
        &mut self,
        name: &str,
        id: &SessionId,
        arguments: Value,
    ) -> Value {
        let response = self.session(name, id, arguments).await;
        let state = self.settle(id, response["state"].clone()).await;
        assert_completion(&state, response.get("job_id"));
        state
    }

    pub(in super::super) async fn keys(&mut self, id: &SessionId, keys: &[&str]) -> Value {
        self.perform("ffm_session_input", id, json!({"keys":keys}))
            .await
    }

    pub(in super::super) async fn text(&mut self, id: &SessionId, text: &str) -> Value {
        self.perform("ffm_session_input", id, json!({"text":text}))
            .await
    }

    pub(in super::super) async fn action(
        &mut self,
        id: &SessionId,
        context: &str,
        action: &str,
    ) -> Value {
        self.perform(
            "ffm_session_action",
            id,
            json!({"context":context,"action":action}),
        )
        .await
    }

    pub(in super::super) async fn close_session(&mut self, id: &SessionId) {
        self.session("ffm_session_close", id, json!({})).await;
    }

    pub(in super::super) async fn select(&mut self, id: &SessionId, name: &str) {
        let state = self.state(id).await;
        let index = state["panels"]["left"]["entries"]
            .as_array()
            .expect("panel entries")
            .iter()
            .position(|entry| entry["name"] == name)
            .expect("fixture is listed");
        let mut keys = vec!["Home"];
        keys.extend(std::iter::repeat_n("Down", index));
        self.keys(id, &keys).await;
    }
}

fn assert_completion(state: &Value, accepted_id: Option<&Value>) {
    let job = state
        .get("job")
        .expect("completed state includes job details");
    assert_eq!(job.get("error"), Some(&Value::Null), "{state}");
    if let Some(accepted_id) = accepted_id {
        assert_eq!(job.get("status").and_then(Value::as_str), Some("finished"));
        assert_eq!(
            job.get("id"),
            Some(accepted_id),
            "completion belongs to the accepted job"
        );
    }
}
