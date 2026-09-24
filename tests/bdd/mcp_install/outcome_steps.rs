//! Purpose: Then steps: what each client's configuration holds after registering.

use super::*;

#[then(expr = "the claude CLI ran {string} with the server")]
fn claude_ran(world: &mut FfmWorld, args: String) -> StepResult {
    let mcp = scenario(world)?;
    let mut expected: Vec<String> = args.split_whitespace().map(str::to_owned).collect();
    expected.push(mcp.server()?.as_str().to_owned());
    let calls = mcp.machine.claude().calls();
    assert_eq!(calls.len(), 1, "{calls:?}");
    let call = calls.first().context("one claude call")?;
    assert_eq!(call.args, expected);
    assert_eq!(call.cwd, mcp.machine.home());
    Ok(())
}

#[then("the claude CLI was not run")]
fn claude_not_run(world: &mut FfmWorld) -> StepResult {
    let calls = scenario(world)?.machine.claude().calls();
    assert!(calls.is_empty(), "{calls:?}");
    Ok(())
}

#[then("the user-level Codex, Junie and Kiro configs register ffm-mcp without FFM_MCP_ROOT")]
fn user_configs_registered(world: &mut FfmWorld) -> StepResult {
    let mcp = scenario(world)?;
    for client in [ClientId::Codex, ClientId::Junie, ClientId::Kiro] {
        assert_eq!(
            mcp.registered(client, Scope::AllProjects)?,
            ConfigState::Registered,
            "{client:?}: {}",
            mcp.read(client, Scope::AllProjects)?
        );
    }
    Ok(())
}

#[then("the user-level Junie config registers ffm-mcp without FFM_MCP_ROOT")]
fn junie_registered(world: &mut FfmWorld) -> StepResult {
    let mcp = scenario(world)?;
    assert_eq!(
        mcp.registered(ClientId::Junie, Scope::AllProjects)?,
        ConfigState::Registered
    );
    Ok(())
}

#[then(
    "the folder's .mcp.json, .codex/config.toml, .junie/mcp/mcp.json and .kiro/settings/mcp.json register ffm-mcp with FFM_MCP_ROOT set to the folder"
)]
fn folder_configs_registered(world: &mut FfmWorld) -> StepResult {
    let mcp = scenario(world)?;
    for client in [
        ClientId::ClaudeCode,
        ClientId::Codex,
        ClientId::Junie,
        ClientId::Kiro,
    ] {
        assert_eq!(
            mcp.registered(client, Scope::CurrentFolder)?,
            ConfigState::Registered,
            "{client:?}: {}",
            mcp.read(client, Scope::CurrentFolder)?
        );
    }
    Ok(())
}

#[then(expr = "the user-level Kiro config still contains exactly {string}")]
fn kiro_untouched(world: &mut FfmWorld, text: String) -> StepResult {
    assert_eq!(
        scenario(world)?.read(ClientId::Kiro, Scope::AllProjects)?,
        text
    );
    Ok(())
}

#[then(expr = "the user-level Junie config still holds the server {string} and the key {string}")]
fn junie_kept(world: &mut FfmWorld, server: String, key: String) -> StepResult {
    let text = scenario(world)?.read(ClientId::Junie, Scope::AllProjects)?;
    let config: Value = serde_json::from_str(&text).context("JSON")?;
    assert_eq!(field(&config, &key), "dark", "{text}");
    let servers = field(&config, "mcpServers");
    assert_eq!(field(field(servers, &server), "command"), "other", "{text}");
    assert!(field(servers, "ffm-mcp").is_object(), "{text}");
    Ok(())
}

/// `value[key]` without indexing: `Null` when `key` is missing.
fn field<'a>(value: &'a Value, key: &str) -> &'a Value {
    value.get(key).unwrap_or(&Value::Null)
}

#[then("the user-level Junie config was written once")]
fn junie_written_once(world: &mut FfmWorld) -> StepResult {
    let stamps = &scenario(world)?.junie_stamps;
    assert_eq!(stamps.len(), 3, "before and two runs: {stamps:?}");
    let [before, first, second] = stamps.as_slice() else {
        return Err(format!("before and two runs: {stamps:?}"));
    };
    assert_ne!(before, first, "the first run writes");
    assert_eq!(first, second, "the second run writes nothing");
    Ok(())
}

#[then(
    expr = "the user-level Codex config registers ffm-mcp at the server with startup_timeout_sec {int}"
)]
fn codex_updated(world: &mut FfmWorld, timeout: u32) -> StepResult {
    let mcp = scenario(world)?;
    let text = mcp.read(ClientId::Codex, Scope::AllProjects)?;
    assert_eq!(
        mcp.registered(ClientId::Codex, Scope::AllProjects)?,
        ConfigState::Registered,
        "{text}"
    );
    assert!(
        text.contains(&format!("startup_timeout_sec = {timeout}")),
        "{text}"
    );
    assert!(text.starts_with("# Codex settings\n"), "{text}");
    Ok(())
}

#[then("no user-level Codex, Junie or Kiro config was written")]
fn no_user_configs(world: &mut FfmWorld) -> StepResult {
    let mcp = scenario(world)?;
    for client in [ClientId::Codex, ClientId::Junie, ClientId::Kiro] {
        assert!(
            !mcp.config(client).exists(),
            "{client:?}: {}",
            mcp.read(client, Scope::AllProjects)?
        );
    }
    Ok(())
}

#[then("no config file was created or changed")]
fn nothing_written(world: &mut FfmWorld) -> StepResult {
    let mcp = scenario(world)?;
    assert_eq!(mcp.machine.files().context("snapshot")?, mcp.before);
    Ok(())
}
