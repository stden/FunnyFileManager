//! Unit tests for the JSON configs: reading the state of `mcpServers` and
//! setting the `ffm-mcp` entry without disturbing anything else.

use serde_json::{json, Value};

use super::tests::{all_projects, current_folder, folder, server};
use super::*;

fn json_state(text: &str, desired: &ServerEntry) -> ConfigState {
    state(ConfigFormat::Json, text, Some(desired))
}

fn upserted(text: &str, desired: &ServerEntry) -> String {
    upsert(ConfigFormat::Json, text, desired).unwrap()
}

fn parsed(text: &str) -> Value {
    serde_json::from_str(text.trim_start_matches('\u{feff}')).unwrap()
}

#[test]
fn empty_whitespace_and_bom_only_files_are_not_configured() {
    for text in ["", "  \n\t", "\u{feff}", "\u{feff}\n"] {
        assert_eq!(
            json_state(text, &all_projects()),
            ConfigState::NotConfigured
        );
    }
}

#[test]
fn a_file_that_does_not_parse_names_the_line_and_column() {
    let ConfigState::Blocked(Blocker::Unparsable(message)) =
        json_state("{\n  \"a\": 1,\n", &all_projects())
    else {
        panic!("expected a parse error");
    };
    assert!(message.contains("line"), "{message}");
    assert!(message.contains("column"), "{message}");
}

#[test]
fn unexpected_shapes_name_the_key_that_is_not_an_object() {
    let cases = [
        ("[]", "the top level"),
        (r#"{"mcpServers": "x"}"#, "mcpServers"),
        (r#"{"mcpServers": {"ffm-mcp": []}}"#, "mcpServers.ffm-mcp"),
        (
            r#"{"mcpServers": {"ffm-mcp": {"env": 3}}}"#,
            "mcpServers.ffm-mcp.env",
        ),
    ];
    for (text, key) in cases {
        let blocked = ConfigState::Blocked(Blocker::UnexpectedLayout(key));
        assert_eq!(json_state(text, &all_projects()), blocked, "{text}");
        assert_eq!(
            upsert(ConfigFormat::Json, text, &all_projects()),
            Err(Blocker::UnexpectedLayout(key)),
            "{text}"
        );
    }
}

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
fn other_servers_and_top_level_keys_survive_in_order() {
    let text = r#"{
  "theme": "dark",
  "mcpServers": {
    "zeta": {"command": "z"},
    "alpha": {"command": "a"}
  },
  "after": 1
}"#;
    let written = upserted(text, &all_projects());
    let order = [
        "\"theme\"",
        "\"mcpServers\"",
        "\"zeta\"",
        "\"alpha\"",
        "\"ffm-mcp\"",
        "\"after\"",
    ];
    let positions: Vec<usize> = order
        .iter()
        .map(|key| written.find(key).unwrap_or_else(|| panic!("{key} lost")))
        .collect();
    let mut sorted = positions.clone();
    sorted.sort_unstable();
    assert_eq!(positions, sorted, "{written}");
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
fn a_bom_is_kept() {
    let written = upserted("\u{feff}{}", &all_projects());
    assert!(written.starts_with('\u{feff}'));
    assert!(written.ends_with("}\n"));
    assert!(!upserted("{}", &all_projects()).starts_with('\u{feff}'));
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
fn a_file_that_does_not_parse_is_never_rewritten() {
    assert!(matches!(
        upsert(ConfigFormat::Json, "{ broken", &all_projects()),
        Err(Blocker::Unparsable(_))
    ));
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

/// Review finding: a `Value` round trip rewrote other people's numbers
/// (`1e3` became `1000.0`, a 24-digit integer lost its tail) and turned the
/// file's CRLF line ends into LF.
#[test]
fn every_other_value_keeps_its_exact_text_and_the_line_ends() {
    let text = "{\r\n  \"big\": 123456789012345678901234,\r\n  \"exp\": 1e3,\r\n  \
                \"pi\": 3.14159265358979323846,\r\n  \"nested\": {\"a\":1,  \"b\":[1e-7]},\r\n  \
                \"mcpServers\": {\r\n    \"other\": {\"command\":\"x\",\"n\":2E+2}\r\n  }\r\n}\r\n";
    let written = upserted(text, &all_projects());
    for kept in [
        "\"big\": 123456789012345678901234,",
        "\"exp\": 1e3,",
        "\"pi\": 3.14159265358979323846,",
        "\"nested\": {\"a\":1,  \"b\":[1e-7]},",
        "\"other\": {\"command\":\"x\",\"n\":2E+2},",
    ] {
        assert!(written.contains(kept), "lost {kept:?}:\n{written}");
    }
    assert!(!written.replace("\r\n", "").contains('\n'), "{written:?}");
    assert!(written.ends_with("}\r\n"), "{written:?}");
    assert_eq!(
        json_state(&written, &all_projects()),
        ConfigState::Registered
    );
    assert_eq!(upserted(&written, &all_projects()), written);
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

/// The kept values keep their lines, so the rewrite indents as the file does.
#[test]
fn a_file_indented_by_four_keeps_four() {
    let text = "{\n    \"theme\": {\n        \"a\": 1\n    },\n    \"mcpServers\": {\n        \"other\": {\n            \"command\": \"x\"\n        }\n    }\n}\n";
    let expected = format!(
        "{{\n    \"theme\": {{\n        \"a\": 1\n    }},\n    \"mcpServers\": {{\n        \"other\": {{\n            \"command\": \"x\"\n        }},\n        \"ffm-mcp\": {{\n            \"command\": {:?},\n            \"args\": [],\n            \"env\": {{}}\n        }}\n    }}\n}}\n",
        server().as_str()
    );
    assert_eq!(upserted(text, &all_projects()), expected);
}
