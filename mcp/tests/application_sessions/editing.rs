//! Purpose: Exercise real keyboard input, named editor dispatch and independent child state.

use super::client::{Access, Client};
use serde_json::Value;
use std::{fs, path::Path};

#[tokio::test]
#[ignore = "requires FFM_APPLICATION_TEST_BINARY pointing to a separately built ffm executable"]
async fn f4_then_named_editor_save_changes_real_file_bytes() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("sample.txt");
    fs::write(&path, "alpha\n").unwrap();
    let mut client = Client::spawn(root.path(), Access::Application).await;
    let id = client.start(root.path()).await;
    client.select(&id, "sample.txt").await;
    let state = client.keys(&id, &["F4"]).await;
    assert_eq!(state["modal"]["context"], "editor");
    assert!(screen(&state).contains("alpha"));
    client.keys(&id, &["Ctrl+Home"]).await;
    client.text(&id, "prefix ").await;
    assert_eq!(fs::read(&path).unwrap(), b"alpha\n");
    let saved = client.action(&id, "editor", "save").await;
    assert_eq!(fs::read(&path).unwrap(), b"prefix alpha\n");
    assert_eq!(saved["editor"]["modified"], false);
    client.close_session(&id).await;
    client.close().await;
}

#[tokio::test]
#[ignore = "requires FFM_APPLICATION_TEST_BINARY pointing to a separately built ffm executable"]
async fn two_sessions_keep_their_panels_and_modals_independent() {
    let root = tempfile::tempdir().unwrap();
    let (left, right) = (root.path().join("left"), root.path().join("right"));
    fs::create_dir(&left).unwrap();
    fs::create_dir(&right).unwrap();
    let mut client = Client::spawn(root.path(), Access::Application).await;
    let first = client.start(&left).await;
    let second = client.start(&right).await;
    let first_state = client.keys(&first, &["F1"]).await;
    let second_state = client.state(&second).await;
    assert_eq!(first_state["modal"]["kind"], "help");
    assert_eq!(second_state["modal"]["kind"], "none");
    assert_path(&first_state["panels"]["left"]["path"], &left);
    assert_path(&second_state["panels"]["left"]["path"], &right);
    client.close_session(&first).await;
    assert_eq!(client.state(&second).await["running"], true);
    client.close_session(&second).await;
    client.close().await;
}

fn screen(state: &Value) -> String {
    state["screen"]["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row.as_str().unwrap())
        .collect::<Vec<_>>()
        .join("\n")
}

fn assert_path(value: &Value, expected: &Path) {
    assert_eq!(
        Path::new(value.as_str().unwrap()).canonicalize().unwrap(),
        expected.canonicalize().unwrap()
    );
}
