//! Purpose: Steps for `tests/features/mcp_install.feature` — F9 Commands >
//! Install MCP server driven through the real event loop, on a fake machine
//! whose files and `claude` calls the steps then read.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use crossterm::event::{Event, KeyCode};
use cucumber::{given, then, when};
use funny_file_manager::mcp_clients::{
    self, ClientId, ConfigState, ProjectDir, Scope, ServerBinary, ServerEntry,
};
use funny_file_manager::models::{App, Panel};
use funny_file_manager::test_support::{file_identity, ClaudeBehaviour, FakeMachine};
use serde_json::Value;

use super::bdd_runner::{Context as _, StepResult};
use super::{key_event, push_quit, FfmWorld};

/// The fake machine and what the steps remember about it between runs.
#[derive(Debug)]
pub struct McpScenario {
    machine: FakeMachine,
    /// Every file before the first registration.
    before: BTreeMap<PathBuf, Vec<u8>>,
    /// The Junie user config's identity — its inode, which every atomic
    /// rewrite changes where a modification time may not: before, then after
    /// each run.
    junie_stamps: Vec<Option<Stamp>>,
    /// The folder the panels show; the project folder unless a step moves it.
    panel: Option<PathBuf>,
}

/// What `file_identity` returns on this platform.
#[cfg(unix)]
type Stamp = u64;
#[cfg(not(unix))]
type Stamp = std::time::SystemTime;

impl McpScenario {
    fn junie_stamp(&self) -> Option<Stamp> {
        file_identity(&self.config(ClientId::Junie)).ok()
    }

    fn config(&self, client: ClientId) -> PathBuf {
        self.machine.config(client, Scope::AllProjects)
    }

    fn read(&self, client: ClientId, scope: Scope) -> StepResult<String> {
        fs::read_to_string(self.machine.config(client, scope)).context("the config was written")
    }

    fn server(&self) -> StepResult<ServerBinary> {
        ServerBinary::new(self.machine.server()).context("a Unicode temp path")
    }

    /// Whether `client`'s config at `scope` registers this server, rooted as
    /// the scope asks: the check the installer itself makes.
    fn registered(&self, client: ClientId, scope: Scope) -> StepResult<ConfigState> {
        let root = match scope {
            Scope::AllProjects => None,
            Scope::CurrentFolder => ProjectDir::new(self.machine.folder()),
        };
        let desired = ServerEntry {
            command: self.server()?,
            root,
        };
        let format = client.client().format;
        Ok(mcp_clients::state(
            format,
            &self.read(client, scope)?,
            Some(&desired),
        ))
    }
}

fn scenario(world: &mut FfmWorld) -> StepResult<&mut McpScenario> {
    world.mcp.as_mut().context("no fake machine")
}

#[given("a fake machine with Claude Code, Codex, Junie and Kiro detected")]
fn fake_machine(world: &mut FfmWorld) -> StepResult {
    let machine = FakeMachine::new().context("fake machine")?;
    machine.install_clients().context("clients on PATH")?;
    world.mcp = Some(McpScenario {
        machine,
        before: BTreeMap::new(),
        junie_stamps: Vec::new(),
        panel: None,
    });
    Ok(())
}

#[given("the panels show the home folder")]
fn panels_at_home(world: &mut FfmWorld) -> StepResult {
    let mcp = scenario(world)?;
    mcp.panel = Some(mcp.machine.home());
    Ok(())
}

#[given("ffm-mcp next to ffm")]
fn server_next_to_ffm(world: &mut FfmWorld) -> StepResult {
    scenario(world)?
        .machine
        .install_server()
        .context("the server")?;
    Ok(())
}

#[given("no ffm-mcp next to ffm or on PATH")]
fn no_server(world: &mut FfmWorld) -> StepResult {
    scenario(world)?.machine.remove_server().context("remove")
}

#[given("the claude CLI times out")]
fn claude_times_out(world: &mut FfmWorld) -> StepResult {
    scenario(world)?
        .machine
        .claude()
        .behave(ClaudeBehaviour::TimeOut);
    Ok(())
}

#[given(expr = "the user-level Kiro config contains {string}")]
fn kiro_config(world: &mut FfmWorld, text: String) -> StepResult {
    scenario(world)?
        .machine
        .write_config(ClientId::Kiro, Scope::AllProjects, &text)
        .context("write")?;
    Ok(())
}

#[given(expr = "the user-level Junie config holds another server {string} and a key {string}")]
fn junie_config(world: &mut FfmWorld, server: String, key: String) -> StepResult {
    let text = serde_json::json!({
        key: "dark",
        "mcpServers": {server: {"command": "other"}}
    });
    scenario(world)?
        .machine
        .write_config(ClientId::Junie, Scope::AllProjects, &format!("{text:#}\n"))
        .context("write")?;
    Ok(())
}

#[given(
    expr = "the user-level Codex config registers ffm-mcp at {string} with startup_timeout_sec {int}"
)]
fn codex_config(world: &mut FfmWorld, command: String, timeout: u32) -> StepResult {
    let text = format!(
        "# Codex settings\n[mcp_servers.ffm-mcp]\ncommand = {command:?}\nstartup_timeout_sec = {timeout}\n"
    );
    scenario(world)?
        .machine
        .write_config(ClientId::Codex, Scope::AllProjects, &text)
        .context("write")?;
    Ok(())
}

/// Open F9, walk to Commands, press the row's hotkey, optionally turn the
/// scope radio, and press Enter; then close whatever is open and quit.
fn register(world: &mut FfmWorld, scope: Scope) -> StepResult {
    let mcp = scenario(world)?;
    if mcp.junie_stamps.is_empty() {
        mcp.before = mcp.machine.files().context("snapshot")?;
        mcp.junie_stamps.push(mcp.junie_stamp());
    }
    let folder = mcp.panel.clone().unwrap_or_else(|| mcp.machine.folder());
    let mut app = App::with_panels(
        Panel::new(folder.clone()).context("panel")?,
        Panel::new(folder).context("panel")?,
    );
    app.live.mcp_host = mcp.machine.host();
    world.app = Some(app);

    let mut events: Vec<Event> = [KeyCode::F(9), KeyCode::Right, KeyCode::Right]
        .into_iter()
        .chain([KeyCode::Char('e')])
        .chain((scope == Scope::CurrentFolder).then_some(KeyCode::Right))
        .chain([KeyCode::Enter, KeyCode::Esc])
        .map(key_event)
        .collect();
    push_quit(&mut events);
    world.run_key_workflow(events, "Install MCP server")?;

    let mcp = scenario(world)?;
    let stamp = mcp.junie_stamp();
    mcp.junie_stamps.push(stamp);
    Ok(())
}

#[when("I register the MCP server for all projects")]
fn register_all_projects(world: &mut FfmWorld) -> StepResult {
    register(world, Scope::AllProjects)
}

#[when("I register the MCP server for the current folder")]
fn register_current_folder(world: &mut FfmWorld) -> StepResult {
    register(world, Scope::CurrentFolder)
}

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
