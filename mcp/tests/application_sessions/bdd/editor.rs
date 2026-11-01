//! Purpose: User-visible editor and isolation steps execute canonical FFM actions.

use super::world::{Role, SessionWorld};
use cucumber::{then, when};
use serde_json::Value;

#[when(expr = "the {word} session opens the draft in its editor")]
async fn open_draft(world: &mut SessionWorld, role: String) {
    let (client, id) = world.case().session(Role::named(&role));
    client.select(&id, "draft.txt").await;
    let state = client.keys(&id, &["F4"]).await;
    assert_eq!(state["modal"]["context"], "editor");
}

#[when(expr = "the {word} editor inserts {string} at the start")]
async fn insert(world: &mut SessionWorld, role: String, text: String) {
    let (client, id) = world.case().session(Role::named(&role));
    client.keys(&id, &["Ctrl+Home"]).await;
    client.text(&id, &text).await;
}

#[when(expr = "the {word} editor invokes its named save action")]
async fn save(world: &mut SessionWorld, role: String) {
    let (client, id) = world.case().session(Role::named(&role));
    let state = client.action(&id, "editor", "save").await;
    assert_eq!(state["editor"]["modified"], false);
}

#[then(expr = "the draft on disk contains {string} followed by a newline")]
fn disk_bytes(world: &mut SessionWorld, text: String) {
    assert_eq!(
        std::fs::read(world.case().root.path().join("draft.txt")).unwrap(),
        format!("{text}\n").as_bytes()
    );
}

#[then(expr = "the {word} editor displays {string}")]
async fn displays(world: &mut SessionWorld, role: String, text: String) {
    let (client, id) = world.case().session(Role::named(&role));
    let state = client.state(&id).await;
    assert!(screen(&state).contains(&text));
}

#[then(expr = "the {word} editor does not display {string}")]
async fn does_not_display(world: &mut SessionWorld, role: String, text: String) {
    let (client, id) = world.case().session(Role::named(&role));
    let state = client.state(&id).await;
    assert!(!screen(&state).contains(&text));
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
