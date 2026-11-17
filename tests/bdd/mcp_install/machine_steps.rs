//! Purpose: Given steps: the fake machine, the server binary and each client's existing
//! configuration.

use super::*;

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
