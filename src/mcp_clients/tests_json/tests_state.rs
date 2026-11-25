//! Purpose: The state a JSON config reads as: not configured, registered,
//! a different entry, disabled, or started some other way.

use super::*;

#[test]
fn a_file_without_the_entry_is_not_configured() {
    let text = r#"{"mcpServers": {"other": {"command": "x"}}}"#;
    assert_eq!(
        json_state(text, &all_projects()),
        ConfigState::NotConfigured
    );
    assert_eq!(
        json_state("{}", &all_projects()),
        ConfigState::NotConfigured
    );
}

#[test]
fn a_matching_entry_is_registered_whatever_else_it_carries() {
    let text = json!({"mcpServers": {"ffm-mcp": {
        "type": "stdio", "command": server().as_str(), "args": [],
        "env": {"RUST_LOG": "debug"}, "disabled": false, "autoApprove": []
    }}})
    .to_string();
    assert_eq!(json_state(&text, &all_projects()), ConfigState::Registered);
}

#[test]
fn another_command_arguments_or_root_is_a_different_entry() {
    let entries = [
        json!({"command": "/old/ffm-mcp"}),
        json!({"command": server().as_str(), "args": ["--verbose"]}),
        json!({"command": server().as_str(), "env": {"FFM_MCP_ROOT": "/srv/x/other"}}),
    ];
    for entry in entries {
        let text = json!({"mcpServers": {"ffm-mcp": entry}}).to_string();
        assert_eq!(
            json_state(&text, &all_projects()),
            ConfigState::DifferentEntry
        );
    }
}

/// Review finding: `"disabled": true` read as plain "registered".
#[test]
fn a_switched_off_entry_is_registered_but_disabled() {
    for switch in [json!({"disabled": true}), json!({"enabled": false})] {
        let mut entry = json!({"command": server().as_str()});
        entry
            .as_object_mut()
            .unwrap()
            .extend(switch.as_object().unwrap().clone());
        let text = json!({"mcpServers": {"ffm-mcp": entry}}).to_string();
        assert_eq!(
            json_state(&text, &all_projects()),
            ConfigState::Disabled,
            "{text}"
        );
    }
}

/// Review finding: an entry with a `url`, a `cwd` or an HTTP `type` read as
/// registered, dimmed, and could not be fixed from the dialog.
#[test]
fn an_entry_started_some_other_way_is_a_different_entry() {
    let command = server().as_str().to_owned();
    for extra in [
        json!({"type": "http", "url": "http://localhost:1"}),
        json!({"url": "http://localhost:1"}),
        json!({"cwd": "/srv/x/elsewhere"}),
        json!({"type": 7}),
    ] {
        let mut entry = json!({"command": command});
        entry
            .as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        let text = json!({"mcpServers": {"ffm-mcp": entry}}).to_string();
        assert_eq!(
            json_state(&text, &all_projects()),
            ConfigState::DifferentEntry,
            "{text}"
        );
        let written = upserted(&text, &all_projects());
        assert_eq!(
            json_state(&written, &all_projects()),
            ConfigState::Registered,
            "{written}"
        );
    }
}
