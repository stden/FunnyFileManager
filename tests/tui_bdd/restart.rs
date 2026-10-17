//! Restart the real event loop using an isolated persistent panel session.

use std::fs;

use cucumber::{then, when};
use ratatui::layout::Size;

use super::{session::Session, TuiWorld};

#[when(expr = "I resume the TUI at {int} by {int}")]
fn resume(world: &mut TuiWorld, width: u16, height: u16) {
    world.finish();
    let workspace = world.root();
    world.session = Some(Session::resume(
        workspace,
        workspace.join("config/panel-session.json"),
        Size::new(width, height),
    ));
}

#[then(expr = "the saved panel directories are {string} and {string}")]
#[expect(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a Cucumber step fails by panicking, as a #[test] does"
)]
#[expect(
    clippy::needless_pass_by_value,
    clippy::needless_pass_by_ref_mut,
    reason = "Cucumber dictates step signatures: captured args arrive owned and the world as `&mut`"
)]
fn saved(world: &mut TuiWorld, left: String, right: String) {
    let workspace = world.root();
    let bytes = fs::read(workspace.join("config/panel-session.json")).expect("saved session");
    let paths: std::collections::BTreeMap<String, std::path::PathBuf> =
        serde_json::from_slice(&bytes).expect("session JSON");
    assert_eq!(
        paths["left"].canonicalize().unwrap(),
        world.path(&left, "").canonicalize().unwrap()
    );
    assert_eq!(
        paths["right"].canonicalize().unwrap(),
        world.path(&right, "").canonicalize().unwrap()
    );
}

#[when(expr = "the {word} workspace directory is removed")]
#[expect(
    clippy::expect_used,
    reason = "a Cucumber step fails by panicking, as a #[test] does"
)]
#[expect(
    clippy::needless_pass_by_value,
    clippy::needless_pass_by_ref_mut,
    reason = "Cucumber dictates step signatures: captured args arrive owned and the world as `&mut`"
)]
fn remove_directory(world: &mut TuiWorld, side: String) {
    // The path comes solely from this scenario's TempDir and fixed side names.
    fs::remove_dir_all(world.path(&side, "")).expect("remove fixture directory");
}
