//! Purpose: Discover every MCP tool and canonical FFM action through real processes.

use super::client::{Access, Client};
use serde_json::{json, Value};
use std::collections::BTreeSet;

#[tokio::test]
#[ignore = "requires FFM_APPLICATION_TEST_BINARY pointing to a separately built ffm executable"]
async fn all_tools_and_fourteen_application_contexts_are_discoverable() {
    let root = tempfile::tempdir().unwrap();
    let mut client = Client::spawn(root.path(), Access::Application).await;
    let tools = client.rpc("tools/list", json!({})).await;
    assert_eq!(tools["result"]["tools"].as_array().unwrap().len(), 63);
    let id = client.start(root.path()).await;
    let catalog = client.session("ffm_session_catalog", &id, json!({})).await;
    let actions = catalog["actions"]
        .as_array()
        .expect("canonical action catalog");
    assert_actions(actions);
    assert_contexts(actions);
    let resized = client
        .perform("ffm_session_resize", &id, json!({"width":100,"height":30}))
        .await;
    assert_eq!(resized["screen"]["width"], 100);
    assert_eq!(resized["screen"]["rows"].as_array().unwrap().len(), 30);
    client.close_session(&id).await;
    client.close().await;
}

fn assert_actions(actions: &[Value]) {
    assert!(
        actions.len() >= 329,
        "the complete baseline remains discoverable"
    );
    let identities: BTreeSet<_> = actions
        .iter()
        .map(|row| {
            (
                row["context"].as_str().unwrap(),
                row["action"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        identities.len(),
        actions.len(),
        "catalog identities are unique"
    );
    for identity in [
        ("panels", "edit"),
        ("editor", "save"),
        ("panels", "system_temp_cleanup"),
        ("panels", "space_analysis"),
        ("panels", "things_catalog"),
    ] {
        assert!(
            identities.contains(&identity),
            "missing action {identity:?}"
        );
    }
}

fn assert_contexts(actions: &[Value]) {
    let contexts: BTreeSet<_> = actions
        .iter()
        .map(|row| row["context"].as_str().unwrap())
        .collect();
    let expected: BTreeSet<_> = [
        "panels",
        "tree_panel",
        "netbox_panel",
        "command_line",
        "command_line_search",
        "command_line_completion",
        "viewer",
        "editor",
        "task_list",
        "task_list_search",
        "wsl_manager",
        "language_servers",
        "path_editor",
        "vault",
    ]
    .into_iter()
    .collect();
    assert_eq!(contexts, expected);
}
