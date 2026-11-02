//! Purpose: A real shell job never inherits protocol streams and remains cancellable.

use super::client::{Access, Client, SessionId};
use serde_json::{json, Value};
use std::time::{Duration, Instant};

#[tokio::test]
#[ignore = "requires FFM_APPLICATION_TEST_BINARY pointing to a separately built ffm executable"]
async fn shell_output_stays_in_state_while_protocol_calls_and_cancellation_stay_responsive() {
    let root = tempfile::tempdir().unwrap();
    let mut client = Client::spawn(root.path(), Access::Application).await;
    let id = client.start(root.path()).await;
    client.action(&id, "panels", "command_line").await;
    client.text(&id, command()).await;
    let accepted = client
        .session("ffm_session_input", &id, json!({"keys":["Enter"]}))
        .await;
    assert!(
        accepted["job_id"].is_number(),
        "long actions acknowledge a job promptly"
    );
    let accepted_id = accepted["job_id"].clone();
    let running = wait_for_output(&mut client, &id).await;
    assert_eq!(running["busy"], true);
    assert_eq!(running["external_process"]["state"], "running");
    let response = tokio::time::timeout(Duration::from_secs(2), client.state(&id)).await;
    assert!(
        response.is_ok(),
        "state must remain responsive during the shell job"
    );
    let cancelled = client.session("ffm_session_cancel", &id, json!({})).await;
    let state = client.settle(&id, cancelled["state"].clone()).await;
    assert_cancelled(&state, &accepted_id);
    client.close_session(&id).await;
    client.close().await;
}

fn assert_cancelled(state: &Value, accepted_id: &Value) {
    assert_eq!(
        &state["job"]["id"], accepted_id,
        "cancellation completes the original job"
    );
    assert_eq!(state["job"]["status"], "finished");
    assert_eq!(state["external_process"]["state"], "cancelled");
    let output = state["external_process"]["output"].as_str().unwrap();
    assert!(!output.contains("ffm-e2e-after"));
}

async fn wait_for_output(client: &mut Client, id: &SessionId) -> Value {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let state = client.state(id).await;
        if state["external_process"]["output"]
            .as_str()
            .unwrap()
            .contains("not_protocol")
        {
            return state;
        }
        assert!(
            Instant::now() < deadline,
            "private shell did not produce its fixture output: {state}"
        );
        tokio::task::yield_now().await;
    }
}

#[cfg(windows)]
fn command() -> &'static str {
    "echo ffm-e2e-start & echo {\"not_protocol\":true} & powershell.exe -NoLogo -NoProfile -NonInteractive -Command \"Start-Sleep -Seconds 30\" & echo ffm-e2e-after"
}

#[cfg(not(windows))]
fn command() -> &'static str {
    "printf 'ffm-e2e-start\\n{\"not_protocol\":true}\\n'; sleep 30; printf 'ffm-e2e-after\\n'"
}
