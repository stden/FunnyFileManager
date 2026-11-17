//! Purpose: The current-folder target, table paths and project folders.

use super::*;

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
