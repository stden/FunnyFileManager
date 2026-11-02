//! Purpose: Given/When/Then steps verify live private input, state and cancellation.

use super::world::{JobId, Role, SessionWorld};
use cucumber::{then, when};
use serde_json::{json, Value};
use std::time::{Duration, Instant};

#[when("the first session starts an interactive fixture shell")]
async fn start(world: &mut SessionWorld) {
    let case = world.case();
    let command = super::shell::command(case.root.path());
    let (client, id) = case.session(Role::First);
    client.action(&id, "panels", "command_line").await;
    client.text(&id, &command).await;
    let accepted = client
        .session("ffm_session_input", &id, json!({"keys":["Enter"]}))
        .await;
    case.job = Some(JobId(
        accepted["job_id"].as_u64().expect("accepted shell job ID"),
    ));
}

#[then(expr = "the private shell output contains {string}")]
async fn output_contains(world: &mut SessionWorld, marker: String) {
    let (client, id) = world.case().session(Role::First);
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let state = client.state(&id).await;
        assert_eq!(
            state["busy"], true,
            "shell stays active for private input/cancellation"
        );
        if output(&state).contains(&marker) {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "missing shell output {marker}: {state}"
        );
        tokio::task::yield_now().await;
    }
}

#[then("MCP state remains responsive while that shell is running")]
async fn responsive(world: &mut SessionWorld) {
    let (client, id) = world.case().session(Role::First);
    let state = tokio::time::timeout(Duration::from_secs(2), client.state(&id))
        .await
        .unwrap();
    assert_eq!(state["external_process"]["state"], "running");
}

#[when("I send fixture input to its private terminal")]
async fn input(world: &mut SessionWorld) {
    let (client, id) = world.case().session(Role::First);
    client
        .session("ffm_session_input", &id, json!({"text":"fixture-reply"}))
        .await;
    client
        .session("ffm_session_input", &id, json!({"keys":["Enter"]}))
        .await;
}

#[when("I cancel the first session's running job")]
async fn cancel(world: &mut SessionWorld) {
    let case = world.case();
    let (client, id) = case.session(Role::First);
    let cancelled = client.session("ffm_session_cancel", &id, json!({})).await;
    case.last = client.settle(&id, cancelled["state"].clone()).await;
}

#[then("the original shell job finishes as cancelled before its final marker")]
fn cancelled(world: &mut SessionWorld) {
    let case = world.case();
    assert_eq!(case.last["job"]["id"].as_u64(), Some(case.job.unwrap().0));
    assert_eq!(case.last["job"]["status"], "finished");
    assert_eq!(case.last["busy"], false);
    assert_eq!(case.last["external_process"]["state"], "cancelled");
    assert!(!output(&case.last).contains("ffm-e2e-after"));
}

fn output(state: &Value) -> &str {
    state["external_process"]["output"].as_str().unwrap()
}
