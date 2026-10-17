//! Purpose: When steps: registering the server for all projects or for the current folder.

use super::*;

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
