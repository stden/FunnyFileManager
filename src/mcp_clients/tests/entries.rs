//! Purpose: How a found entry compares with the desired one, and how `state`
//! and `upsert` dispatch on the config format.

use super::*;

#[test]
fn an_entry_matches_on_command_empty_args_and_root() {
    let found = |command: &str, args, root: Option<&str>| FoundEntry {
        command: Some(command.to_owned()),
        args,
        root: root.map(str::to_owned),
        launch: Launch::AsWritten,
        switch: Switch::On,
    };
    let binary = server();
    let server = binary.as_str();
    let project = folder();
    let project = project.as_str();
    assert!(found(server, Args::Empty, None).matches(&all_projects()));
    assert!(!found(server, Args::Given, None).matches(&all_projects()));
    assert!(!found("/old/ffm-mcp", Args::Empty, None).matches(&all_projects()));
    // A fixed root contradicts "all projects".
    assert!(!found(server, Args::Empty, Some(project)).matches(&all_projects()));
    assert!(found(server, Args::Empty, Some(project)).matches(&current_folder()));
    assert!(!found(server, Args::Empty, Some("/srv/x/other")).matches(&current_folder()));
    assert!(!found(server, Args::Empty, None).matches(&current_folder()));
    let no_command = FoundEntry {
        command: None,
        args: Args::Empty,
        root: None,
        launch: Launch::AsWritten,
        switch: Switch::On,
    };
    assert!(!no_command.matches(&all_projects()));
}

/// Review finding: a `url` or `cwd` entry matched, and a switched-off one
/// read as plain "registered".
#[test]
fn an_entry_started_elsewhere_differs_and_one_switched_off_is_disabled() {
    let entry = |launch, switch| FoundEntry {
        command: Some(server().as_str().to_owned()),
        args: Args::Empty,
        root: None,
        launch,
        switch,
    };
    let desired = all_projects();
    assert!(!entry(Launch::Elsewhere, Switch::On).matches(&desired));
    assert_eq!(
        entry(Launch::Elsewhere, Switch::Off).state(Some(&desired)),
        ConfigState::DifferentEntry
    );
    assert_eq!(
        entry(Launch::AsWritten, Switch::Off).state(Some(&desired)),
        ConfigState::Disabled
    );
    assert_eq!(
        entry(Launch::AsWritten, Switch::On).state(Some(&desired)),
        ConfigState::Registered
    );
}

#[test]
fn state_dispatches_on_the_format() {
    let json = serde_json::json!({"mcpServers": {"ffm-mcp": {"command": server().as_str()}}});
    let json = json.to_string();
    let toml = format!("[mcp_servers.ffm-mcp]\ncommand = {:?}\n", server().as_str());
    assert_eq!(
        state(ConfigFormat::Json, &json, Some(&all_projects())),
        ConfigState::Registered
    );
    assert_eq!(
        state(ConfigFormat::Toml, &toml, Some(&all_projects())),
        ConfigState::Registered
    );
    assert_eq!(
        state(ConfigFormat::Toml, &toml, Some(&current_folder())),
        ConfigState::DifferentEntry
    );
}

/// Review finding: with no server found, a stale entry — even one for a
/// deleted binary — read as "registered".
#[test]
fn with_no_server_to_compare_an_entry_is_only_unverified() {
    let json = r#"{"mcpServers":{"ffm-mcp":{"command":"/old/ffm-mcp"}}}"#;
    assert_eq!(
        state(ConfigFormat::Json, json, None),
        ConfigState::Unverified {
            command: Some("/old/ffm-mcp".to_owned())
        }
    );
    assert_eq!(
        state(
            ConfigFormat::Toml,
            "[mcp_servers.ffm-mcp]\nargs = []\n",
            None
        ),
        ConfigState::Unverified { command: None }
    );
    assert_eq!(
        state(ConfigFormat::Json, "{}", None),
        ConfigState::NotConfigured
    );
}

#[test]
fn upsert_dispatches_on_the_format() {
    let json = upsert(ConfigFormat::Json, "", &all_projects()).unwrap();
    let toml = upsert(ConfigFormat::Toml, "", &all_projects()).unwrap();
    assert!(json.contains("\"mcpServers\""));
    assert!(toml.contains("[mcp_servers.ffm-mcp]"));
}
