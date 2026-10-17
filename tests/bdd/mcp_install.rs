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

#[path = "mcp_install/machine_steps.rs"]
mod machine_steps;
#[path = "mcp_install/outcome_steps.rs"]
mod outcome_steps;
#[path = "mcp_install/register_steps.rs"]
mod register_steps;

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
