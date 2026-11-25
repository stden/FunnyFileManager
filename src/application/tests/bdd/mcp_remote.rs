//! Purpose: Step definitions for Streamable HTTP MCP services acceptance scenarios.

mod fixture;

use cucumber::{given, then, when};

use super::common::{open_command_row, wait_until};
use super::BehaviorWorld;
use crate::mcp_clients::Host;
use crate::menu::MenuCommand;
use crate::models::Overlay;

use fixture::write_project_mcp;
pub(super) use fixture::Fixture;

fn parts(world: &mut BehaviorWorld) -> (&mut super::Case, &mut Fixture) {
    (
        world.case.as_mut().expect("case"),
        world.mcp_remote.as_mut().expect("mcp fixture"),
    )
}

#[given(
    expr = "an HTTP MCP server {string} configured in the active folder with disabled tool {string}"
)]
fn configured_server(world: &mut BehaviorWorld, name: String, disabled: String) {
    let case = world.case();
    case.app.live.mcp_host = Host::default();
    let dir = case.app.get_active_panel_ref().path.clone();
    write_project_mcp(&dir, &name, &disabled);
    world.mcp_remote = Some(Fixture::default());
}

#[given("an HTTP MCP server returning HTTP 403 error")]
fn failing_server(world: &mut BehaviorWorld) {
    let case = world.case();
    case.app.live.mcp_host = Host::default();
    let dir = case.app.get_active_panel_ref().path.clone();
    write_project_mcp(&dir, "alpha", "hidden_tool");
    world.mcp_remote = Some(Fixture::http_error());
}

#[given("a delayed HTTP MCP server response")]
fn delayed_server(world: &mut BehaviorWorld) {
    let case = world.case();
    case.app.live.mcp_host = Host::default();
    let dir = case.app.get_active_panel_ref().path.clone();
    write_project_mcp(&dir, "alpha", "hidden_tool");
    world.mcp_remote = Some(Fixture::delayed());
}

#[when("I open MCP services from the F9 menu")]
fn open(world: &mut BehaviorWorld) {
    let case = world.case();
    case.app.live.mcp_host = Host::default();
    open_command_row(case, MenuCommand::McpServices);
    assert!(matches!(world.case().app.overlay, Overlay::McpServices(_)));
}

#[when("I start the MCP remote worker")]
fn start_worker(world: &mut BehaviorWorld) {
    let (case, fix) = parts(world);
    let work = fix.work();
    fix.jobs.pump_mcp_with(&mut case.app, work);
    assert!(matches!(&case.app.overlay, Overlay::McpServices(d) if d.is_busy()));
    case.paint();
}

#[when("the MCP remote request completes")]
fn complete(world: &mut BehaviorWorld) {
    let (case, fix) = parts(world);
    wait_until("mcp remote request", || {
        let work = fix.work();
        fix.jobs.pump_mcp_with(&mut case.app, work);
        matches!(&case.app.overlay, Overlay::McpServices(d) if !d.is_busy() && d.queued.is_none())
    });
    case.paint();
}

#[when("the live offline MCP worker pumps")]
fn offline_pump(world: &mut BehaviorWorld) {
    let (case, fix) = parts(world);
    wait_until("offline mcp", || {
        let _ = fix.jobs.pump(&mut case.app);
        matches!(&case.app.overlay, Overlay::McpServices(d) if !d.is_busy() && d.queued.is_none())
    });
    case.paint();
}

#[when("I release the delayed MCP remote response")]
fn release(world: &mut BehaviorWorld) {
    let (case, fix) = parts(world);
    fix.gate.release();
    wait_until("delayed mcp reply", || {
        let work = fix.work();
        fix.jobs.pump_mcp_with(&mut case.app, work) == crate::application::Redraw::Needed
    });
    let _ = fix;
    case.paint();
}

#[then(expr = "the MCP transport recorded {int} JSON-RPC requests")]
fn recorded_count(world: &mut BehaviorWorld, expected: usize) {
    let count = world
        .mcp_remote
        .as_ref()
        .expect("fixture")
        .calls
        .lock()
        .expect("lock")
        .len();
    assert_eq!(count, expected);
}

#[then("the reopened MCP dialog still awaits its own tools reply")]
fn reopened_waits(world: &mut BehaviorWorld) {
    let Overlay::McpServices(dialog) = &world.case().app.overlay else {
        panic!("expected MCP services dialog");
    };
    assert!(dialog.is_busy() || dialog.queued.is_some());
    assert_eq!(dialog.tools.len(), 0);
}
