//! Purpose: Given steps: the workspace, its fixture files and a fake MCP machine.

use super::*;

#[given("a TUI workspace")]
#[expect(
    clippy::expect_used,
    reason = "a Cucumber step fails by panicking, as a #[test] does"
)]
fn workspace(world: &mut TuiWorld) {
    let workspace = TempDir::new().expect("workspace");
    for side in ["left", "right"] {
        fs::create_dir(workspace.path().join(side)).expect("panel directory");
    }
    world.workspace = Some(workspace);
}

#[given(expr = "the {word} file {string} contains {string}")]
#[expect(
    clippy::expect_used,
    reason = "a Cucumber step fails by panicking, as a #[test] does"
)]
#[expect(
    clippy::needless_pass_by_value,
    clippy::needless_pass_by_ref_mut,
    reason = "Cucumber dictates step signatures: captured args arrive owned and the world as `&mut`"
)]
fn fixture(world: &mut TuiWorld, side: String, file: String, contents: String) {
    fs::write(world.path(&side, &file), contents).expect("fixture file");
}

#[given("a TUI workspace with a fake MCP machine")]
#[expect(
    clippy::expect_used,
    reason = "a Cucumber step fails by panicking, as a #[test] does"
)]
fn mcp_workspace(world: &mut TuiWorld) {
    workspace(world);
    let machine = funny_file_manager::test_support::FakeMachine::new().expect("fake machine");
    machine.install_clients().expect("clients on PATH");
    machine.install_server().expect("ffm-mcp next to ffm");
    world.mcp = Some(machine);
}
