//! Purpose: Whether a client counts as installed, and homes that block its user-level row.

use super::*;

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
