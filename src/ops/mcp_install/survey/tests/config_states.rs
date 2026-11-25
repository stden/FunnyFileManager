//! Purpose: What a client's config says, and when Claude Code needs its CLI.

use super::*;

#[test]
fn every_state_a_file_client_can_be_in_is_read() {
    let machine = machine();
    machine.install_server().unwrap();
    let host = machine.host();
    let server = machine.server().display().to_string();
    let registered = serde_json::json!({"mcpServers": {"ffm-mcp": {"command": server}}});
    let cases = [
        (None, ConfigState::NoConfig),
        (Some("{}".to_owned()), ConfigState::NotConfigured),
        (Some(registered.to_string()), ConfigState::Registered),
        (
            Some(r#"{"mcpServers": {"ffm-mcp": {"command": "/old"}}}"#.to_owned()),
            ConfigState::DifferentEntry,
        ),
        (
            Some(r#"{"mcpServers": 3}"#.to_owned()),
            ConfigState::Blocked(Blocker::UnexpectedLayout("mcpServers")),
        ),
    ];
    let path = machine.config(ClientId::Kiro, Scope::AllProjects);
    for (text, expected) in cases {
        let _ignored = fs::remove_file(&path);
        if let Some(text) = text {
            machine
                .write_config(ClientId::Kiro, Scope::AllProjects, &text)
                .unwrap();
        }
        let state = state_of(&machine, &host, ClientId::Kiro, Scope::AllProjects);
        assert_eq!(state, expected);
    }
}

#[test]
fn a_config_that_does_not_parse_or_cannot_be_read_is_blocked() {
    let machine = machine();
    machine.install_server().unwrap();
    let host = machine.host();
    machine
        .write_config(ClientId::Codex, Scope::AllProjects, "[mcp_servers")
        .unwrap();
    assert!(matches!(
        state_of(&machine, &host, ClientId::Codex, Scope::AllProjects),
        ConfigState::Blocked(Blocker::Unparsable(_))
    ));
    fs::create_dir_all(machine.config(ClientId::Junie, Scope::AllProjects)).unwrap();
    assert!(matches!(
        state_of(&machine, &host, ClientId::Junie, Scope::AllProjects),
        ConfigState::Blocked(Blocker::Unreadable(_))
    ));
    let kiro = machine
        .write_config(ClientId::Kiro, Scope::AllProjects, "")
        .unwrap();
    fs::write(&kiro, [0xff, 0xfe, 0x00]).unwrap();
    assert!(matches!(
        state_of(&machine, &host, ClientId::Kiro, Scope::AllProjects),
        ConfigState::Blocked(Blocker::Unreadable(message)) if message.contains("UTF-8")
    ));
}

#[test]
fn claude_code_at_all_projects_needs_the_cli_to_change_anything() {
    let machine = machine();
    machine.install_server().unwrap();
    let host = machine.host();
    assert_eq!(
        state_of(&machine, &host, ClientId::ClaudeCode, Scope::AllProjects),
        ConfigState::Blocked(Blocker::NeedsClaudeCli)
    );
    // In a folder FFM writes `.mcp.json` itself.
    assert_eq!(
        state_of(&machine, &host, ClientId::ClaudeCode, Scope::CurrentFolder),
        ConfigState::NoConfig
    );
    machine.install("claude").unwrap();
    assert_eq!(
        state_of(&machine, &host, ClientId::ClaudeCode, Scope::AllProjects),
        ConfigState::NoConfig
    );
}

#[test]
fn an_existing_claude_registration_needs_no_cli() {
    let machine = machine();
    machine.install_server().unwrap();
    let entry = serde_json::json!({"mcpServers": {"ffm-mcp": {
        "type": "stdio", "command": machine.server().display().to_string(), "args": [], "env": {}
    }}});
    fs::write(machine.home().join(".claude.json"), entry.to_string()).unwrap();
    assert_eq!(
        state_of(
            &machine,
            &machine.host(),
            ClientId::ClaudeCode,
            Scope::AllProjects
        ),
        ConfigState::Registered
    );
}

/// Review finding: a missing server made any entry — one for a deleted
/// binary included — read as "registered". Now an entry is only present,
/// and one whose program is an absolute path to nothing says so.
#[test]
fn a_missing_server_leaves_entries_unverified_and_names_a_missing_program() {
    let machine = machine();
    let other = machine.install("other-mcp").unwrap();
    let gone = machine.home().join("deleted").join("ffm-mcp");
    let entry = |command: &Path| {
        serde_json::json!({"mcpServers": {"ffm-mcp": {"command": command}}}).to_string()
    };
    for (client, text) in [
        (ClientId::Junie, entry(&other)),
        (ClientId::Kiro, entry(&gone)),
    ] {
        machine
            .write_config(client, Scope::AllProjects, &text)
            .unwrap();
    }
    let survey = survey(&machine.host(), Scope::AllProjects, None);
    assert_eq!(survey.server, ServerState::Missing);
    assert_eq!(
        status(&survey, ClientId::Junie).state,
        ConfigState::Unverified {
            command: Some(other.to_string_lossy().into_owned())
        }
    );
    assert_eq!(
        status(&survey, ClientId::Kiro).state,
        ConfigState::ProgramMissing
    );
}

/// A bare name is looked up on the client's PATH, which FFM cannot see.
#[test]
fn a_missing_server_leaves_a_bare_command_unverified() {
    let machine = machine();
    let text = "[mcp_servers.ffm-mcp]\ncommand = \"ffm-mcp\"\n";
    machine
        .write_config(ClientId::Codex, Scope::AllProjects, text)
        .unwrap();
    let survey = survey(&machine.host(), Scope::AllProjects, None);
    assert_eq!(
        status(&survey, ClientId::Codex).state,
        ConfigState::Unverified {
            command: Some("ffm-mcp".to_owned())
        }
    );
}
