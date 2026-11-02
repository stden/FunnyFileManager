//! Purpose: Setting the JSON entry: what a new or current-folder entry holds,
//! which keys survive and go, idempotence, and the entry `claude` is given.

use super::*;

#[test]
fn a_new_file_gets_the_whole_entry() {
    let written = parsed(&upserted("", &all_projects()));
    assert_eq!(
        written,
        json!({"mcpServers": {"ffm-mcp": {
            "command": server().as_str(), "args": [], "env": {}
        }}})
    );
}

#[test]
fn the_current_folder_entry_carries_its_root() {
    let written = parsed(&upserted("{}", &current_folder()));
    assert_eq!(
        written["mcpServers"]["ffm-mcp"]["env"]["FFM_MCP_ROOT"],
        folder().as_str()
    );
    assert_eq!(
        json_state(&upserted("{}", &current_folder()), &current_folder()),
        ConfigState::Registered
    );
}

#[test]
fn extra_entry_and_env_keys_survive_and_the_root_goes_at_all_projects() {
    let text = json!({"mcpServers": {"ffm-mcp": {
        "type": "stdio", "command": "/old/ffm-mcp", "args": ["x"], "disabled": true,
        "autoApprove": ["list_directory"],
        "env": {"RUST_LOG": "debug", "FFM_MCP_ROOT": "/srv/x/other", "LAST": "1"}
    }}})
    .to_string();
    let written = parsed(&upserted(&text, &all_projects()));
    let entry = &written["mcpServers"]["ffm-mcp"];
    assert_eq!(entry["type"], "stdio");
    assert_eq!(entry["disabled"], true);
    assert_eq!(entry["autoApprove"], json!(["list_directory"]));
    assert_eq!(entry["command"], server().as_str());
    assert_eq!(entry["args"], json!([]));
    assert_eq!(entry["env"], json!({"RUST_LOG": "debug", "LAST": "1"}));
    let keys: Vec<&String> = entry["env"].as_object().unwrap().keys().collect();
    assert_eq!(
        keys,
        ["RUST_LOG", "LAST"],
        "removing the root keeps the order"
    );
}

#[test]
fn upsert_is_idempotent() {
    let inputs = [
        "",
        r#"{"theme": "dark"}"#,
        r#"{"mcpServers": {"ffm-mcp": {"command": "/old", "args": ["x"]}}}"#,
    ];
    for text in inputs {
        for desired in [all_projects(), current_folder()] {
            let once = upserted(text, &desired);
            assert_eq!(upserted(&once, &desired), once, "{text}");
            assert_eq!(json_state(&once, &desired), ConfigState::Registered);
        }
    }
}

#[test]
fn setting_an_entry_drops_what_starts_it_elsewhere_and_keeps_the_rest() {
    let text = json!({"mcpServers": {"ffm-mcp": {
        "type": "sse", "url": "http://x", "cwd": "/srv/x", "command": "/old",
        "disabled": true, "timeout": 5000
    }}})
    .to_string();
    let written = parsed(&upserted(&text, &all_projects()));
    assert_eq!(
        written["mcpServers"]["ffm-mcp"],
        json!({"command": server().as_str(), "disabled": true, "timeout": 5000,
               "args": [], "env": {}})
    );
    let stdio = json!({"mcpServers": {"ffm-mcp": {"type": "stdio", "command": "/old"}}});
    let written = parsed(&upserted(&stdio.to_string(), &all_projects()));
    assert_eq!(written["mcpServers"]["ffm-mcp"]["type"], "stdio");
}

/// The entry `claude mcp add-json` gets is the one a file edit would leave:
/// the owner's `env` and keys stay, the root and the launch keys go.
#[test]
fn the_claude_entry_is_the_entry_upsert_writes() {
    let text = json!({"oauthAccount": {"id": "x"}, "mcpServers": {"ffm-mcp": {
        "type": "stdio", "command": "/old", "args": ["-v"], "timeout": 5000,
        "env": {"FFM_MCP_TOKEN": "secret", "FFM_MCP_ROOT": "/srv/x/other"}
    }}})
    .to_string();
    let entry: Value =
        serde_json::from_str(&claude_entry(&text, &all_projects()).unwrap()).unwrap();
    assert_eq!(
        entry,
        json!({"type": "stdio", "command": server().as_str(), "args": [], "timeout": 5000,
               "env": {"FFM_MCP_TOKEN": "secret"}})
    );
    assert_eq!(
        parsed(&upserted(&text, &all_projects()))["mcpServers"]["ffm-mcp"],
        entry
    );
    assert!(matches!(
        claude_entry("{ broken", &all_projects()),
        Err(Blocker::Unparsable(_))
    ));
}
