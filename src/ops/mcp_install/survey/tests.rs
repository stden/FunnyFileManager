//! Unit tests for [`super`]: finding programs and the server, telling which
//! clients are installed, and reading every state a config can be in — all on
//! a fake machine.

use crate::test_support::FakeMachine;

use super::*;

fn machine() -> FakeMachine {
    FakeMachine::new().expect("fake machine")
}

fn folder(machine: &FakeMachine) -> ProjectDir {
    project_dir(&machine.folder()).expect("the project folder")
}

fn status(survey: &Survey, client: ClientId) -> &ClientStatus {
    survey
        .clients
        .iter()
        .find(|status| status.client == client)
        .expect("every client is surveyed")
}

fn state_of(machine: &FakeMachine, host: &Host, client: ClientId, scope: Scope) -> ConfigState {
    let survey = survey(host, scope, Some(folder(machine)));
    status(&survey, client).state.clone()
}

#[test]
fn find_program_tries_each_suffix_in_each_folder_and_the_first_folder_wins() {
    let machine = machine();
    let second = machine.home().join("second-bin");
    fs::create_dir(&second).unwrap();
    crate::test_support::write_executable(&second.join("claude.cmd"), b"x").unwrap();
    let first = machine.install("claude").unwrap();
    let mut host = machine.host();
    host.search_path.push(second.clone());
    host.program_suffixes = vec![String::new(), ".cmd".to_owned()];
    assert_eq!(find_program(&host, "claude"), Some(first.clone()));
    fs::remove_file(&first).unwrap();
    assert_eq!(
        find_program(&host, "claude"),
        Some(second.join("claude.cmd"))
    );
}

#[test]
fn find_program_skips_relative_folders_and_an_empty_path() {
    let machine = machine();
    machine.install("claude").unwrap();
    let mut host = machine.host();
    host.search_path = vec![PathBuf::from("relative/bin")];
    assert_eq!(find_program(&host, "claude"), None);
    host.search_path.clear();
    assert_eq!(find_program(&host, "claude"), None);
}

#[cfg(unix)]
#[test]
fn a_file_without_an_execute_bit_is_not_a_program() {
    let machine = machine();
    fs::write(machine.bin().join("codex"), b"not runnable").unwrap();
    assert_eq!(find_program(&machine.host(), "codex"), None);
    fs::create_dir(machine.bin().join("junie")).unwrap();
    assert_eq!(find_program(&machine.host(), "junie"), None);
}

#[test]
fn the_server_next_to_ffm_beats_the_one_on_path() {
    let machine = machine();
    let on_path = machine.install("ffm-mcp").unwrap();
    assert_eq!(
        locate_server(&machine.host()),
        ServerState::Found(ServerBinary::new(on_path).unwrap())
    );
    let beside = machine.install_server().unwrap();
    assert_eq!(
        locate_server(&machine.host()),
        ServerState::Found(ServerBinary::new(beside).unwrap())
    );
}

#[test]
fn no_server_anywhere_is_missing() {
    let machine = machine();
    assert_eq!(locate_server(&machine.host()), ServerState::Missing);
    assert_eq!(locate_server(&Host::default()), ServerState::Missing);
}

#[cfg(unix)]
#[test]
fn a_server_whose_path_is_not_unicode_is_named_but_not_found() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    let machine = machine();
    let odd = machine
        .home()
        .join(OsString::from_vec(b"bin-\xff".to_vec()));
    fs::create_dir(&odd).unwrap();
    crate::test_support::write_executable(&odd.join("ffm-mcp"), b"x").unwrap();
    let mut host = machine.host();
    host.exe_dir = Some(odd.clone());
    assert_eq!(
        locate_server(&host),
        ServerState::NotUnicode(odd.join("ffm-mcp"))
    );
}

#[test]
fn nothing_installed_is_not_detected() {
    let machine = machine();
    let survey = survey(&machine.host(), Scope::AllProjects, None);
    for client in &CLIENTS {
        assert_eq!(status(&survey, client.id).presence, Presence::NotDetected);
    }
}

#[test]
fn a_client_is_detected_by_its_program_on_path() {
    let machine = machine();
    machine.install("kiro-cli").unwrap();
    let survey = survey(&machine.host(), Scope::AllProjects, None);
    assert_eq!(status(&survey, ClientId::Kiro).presence, Presence::Detected);
    assert_eq!(
        status(&survey, ClientId::Codex).presence,
        Presence::NotDetected
    );
}

#[test]
fn a_client_is_detected_by_its_user_config() {
    let machine = machine();
    fs::write(machine.home().join(".claude.json"), "{}").unwrap();
    let survey = survey(&machine.host(), Scope::AllProjects, None);
    assert_eq!(
        status(&survey, ClientId::ClaudeCode).presence,
        Presence::Detected
    );
}

#[test]
fn a_client_is_detected_by_its_home_folder_or_its_override() {
    let machine = machine();
    fs::create_dir(machine.home().join(".junie")).unwrap();
    let elsewhere = machine.home().join("codex-home");
    fs::create_dir(&elsewhere).unwrap();
    let mut host = machine.host();
    host.env.insert("CODEX_HOME", elsewhere.clone().into());
    let survey = survey(&host, Scope::AllProjects, None);
    assert_eq!(
        status(&survey, ClientId::Junie).presence,
        Presence::Detected
    );
    assert_eq!(
        status(&survey, ClientId::Codex).presence,
        Presence::Detected
    );
    assert_eq!(
        status(&survey, ClientId::Codex).config,
        Some(elsewhere.join("config.toml"))
    );
}

#[test]
fn claude_code_is_not_detected_by_the_home_folder_alone() {
    let machine = machine();
    let survey = survey(&machine.host(), Scope::AllProjects, None);
    assert_eq!(
        status(&survey, ClientId::ClaudeCode).presence,
        Presence::NotDetected
    );
}

#[test]
fn an_override_that_is_not_a_folder_blocks_the_user_level_row_only() {
    let machine = machine();
    machine.install_server().unwrap();
    let mut host = machine.host();
    host.env
        .insert("CODEX_HOME", machine.home().join("not-there").into());
    assert_eq!(
        state_of(&machine, &host, ClientId::Codex, Scope::AllProjects),
        ConfigState::Blocked(Blocker::HomeVarNotFolder("CODEX_HOME"))
    );
    assert_eq!(
        state_of(&machine, &host, ClientId::Codex, Scope::CurrentFolder),
        ConfigState::NoConfig
    );
}

#[test]
fn no_home_folder_blocks_every_user_level_row() {
    let machine = machine();
    machine.install_server().unwrap();
    let mut host = machine.host();
    host.home = None;
    let survey = survey(&host, Scope::AllProjects, None);
    for client in &CLIENTS {
        assert_eq!(
            status(&survey, client.id).state,
            ConfigState::Blocked(Blocker::NoHomeFolder)
        );
        assert_eq!(status(&survey, client.id).config, None);
    }
}

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

#[test]
fn the_current_folder_needs_a_folder_and_uses_project_paths() {
    let machine = machine();
    let without = survey(&machine.host(), Scope::CurrentFolder, None);
    assert_eq!(without.scope, Scope::AllProjects);
    let with = survey(
        &machine.host(),
        Scope::CurrentFolder,
        Some(folder(&machine)),
    );
    assert_eq!(with.scope, Scope::CurrentFolder);
    assert_eq!(
        status(&with, ClientId::Junie).config,
        Some(machine.folder().join(".junie").join("mcp").join("mcp.json"))
    );
}

#[test]
fn a_current_folder_entry_must_carry_that_folder_as_its_root() {
    let machine = machine();
    machine.install_server().unwrap();
    let project = folder(&machine);
    let entry = serde_json::json!({"mcpServers": {"ffm-mcp": {
        "command": machine.server().display().to_string(),
        "env": {"FFM_MCP_ROOT": project.as_str()}
    }}});
    machine
        .write_config(
            ClientId::ClaudeCode,
            Scope::CurrentFolder,
            &entry.to_string(),
        )
        .unwrap();
    let host = machine.host();
    assert_eq!(
        state_of(&machine, &host, ClientId::ClaudeCode, Scope::CurrentFolder),
        ConfigState::Registered
    );
}

#[test]
fn a_target_knows_its_scope_root_and_entry() {
    let machine = machine();
    let project = folder(&machine);
    let server = ServerBinary::new(machine.server()).unwrap();
    let here = Target::new(Scope::CurrentFolder, Some(&project));
    assert_eq!(here.scope(), Scope::CurrentFolder);
    assert_eq!(here.root(), Some(&project));
    assert_eq!(here.entry(&server).root, Some(project.clone()));
    let everywhere = Target::new(Scope::AllProjects, Some(&project));
    assert_eq!(everywhere.scope(), Scope::AllProjects);
    assert_eq!(everywhere.entry(&server).root, None);
    assert_eq!(Target::new(Scope::CurrentFolder, None), Target::AllProjects);
}

#[test]
fn table_paths_are_joined_one_part_at_a_time() {
    let base = Path::new("base");
    assert_eq!(
        join_relative(base, ".kiro/settings/mcp.json"),
        base.join(".kiro").join("settings").join("mcp.json")
    );
    assert_eq!(join_relative(base, ""), base);
}

#[test]
fn client_paths_follow_the_table() {
    let machine = machine();
    let host = machine.host();
    let codex = ClientId::Codex.client();
    assert_eq!(client_home(&host, codex), Ok(machine.home().join(".codex")));
    assert_eq!(
        config_path(&host, codex, Target::AllProjects),
        Ok(machine.home().join(".codex").join("config.toml"))
    );
    assert_eq!(
        client_program(&host, codex),
        None,
        "nothing is installed yet"
    );
    assert_eq!(
        read_state(&machine.home().join("none"), ConfigFormat::Json, None),
        ConfigState::NoConfig
    );
    assert_eq!(load(&machine.home().join("none")), Ok(None));
}

#[test]
fn only_an_existing_local_folder_is_a_project() {
    let machine = machine();
    assert!(project_dir(&machine.folder()).is_some());
    assert_eq!(project_dir(&machine.folder().join("gone")), None);
    assert_eq!(project_dir(Path::new("s3://bucket/key")), None);
}

#[test]
fn the_fake_machine_puts_the_server_in_its_executable_folder() {
    let machine = machine();
    let server = machine.install_server().unwrap();
    assert_eq!(server.parent(), Some(machine.exe_dir().as_path()));
    assert_eq!(machine.host().exe_dir, Some(machine.exe_dir()));
}
