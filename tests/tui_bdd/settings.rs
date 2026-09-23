//! The settings file: Save setup writes `settings.yaml`, reads the legacy
//! `settings.json` once, keeps hand edits made while FFM runs, and never
//! overwrites a file that does not parse.

use std::{fs, path::PathBuf};

use cucumber::{gherkin::Step, given, then, when};
use ratatui::layout::Size;

use super::{session::Session, TuiWorld};

/// `name` in the scenario's own config directory, never the user's.
fn settings_file(world: &TuiWorld, name: &str) -> PathBuf {
    world.root().join("config").join(name)
}

/// The step's doc string, one file's text.
#[expect(
    clippy::expect_used,
    reason = "a step written without its doc string must fail here, as a #[test] does"
)]
fn text(step: &Step) -> &str {
    step.docstring.as_deref().expect("a doc string").trim()
}

#[given(expr = "the workspace settings file {string} holds:")]
#[expect(
    clippy::expect_used,
    reason = "a Cucumber step fails by panicking, as a #[test] does"
)]
#[expect(
    clippy::needless_pass_by_value,
    clippy::needless_pass_by_ref_mut,
    reason = "Cucumber dictates step signatures: captured args arrive owned and the world as `&mut`"
)]
fn settings_fixture(world: &mut TuiWorld, name: String, step: &Step) {
    let path = settings_file(world, &name);
    fs::create_dir_all(path.parent().expect("config dir")).expect("config dir");
    fs::write(path, format!("{}\n", text(step))).expect("settings fixture");
}

/// The user changes the file by hand while the TUI is running.
#[when(expr = "the workspace settings file {string} is edited to hold:")]
fn settings_edited(world: &mut TuiWorld, name: String, step: &Step) {
    settings_fixture(world, name, step);
}

#[when(expr = "I open the TUI on the workspace settings at {int} by {int}")]
fn open_with_settings(world: &mut TuiWorld, width: u16, height: u16) {
    world.session = Some(Session::with_settings(
        world.path("left", ""),
        world.path("right", ""),
        settings_file(world, "settings.yaml"),
        Size::new(width, height),
    ));
}

#[then(expr = "the workspace settings file {string} contains:")]
#[expect(
    clippy::expect_used,
    reason = "a Cucumber step fails by panicking, as a #[test] does"
)]
#[expect(
    clippy::needless_pass_by_value,
    clippy::needless_pass_by_ref_mut,
    reason = "Cucumber dictates step signatures: captured args arrive owned and the world as `&mut`"
)]
fn settings_contain(world: &mut TuiWorld, name: String, step: &Step) {
    let saved = fs::read_to_string(settings_file(world, &name)).expect("settings file");
    let expected = text(step);
    assert!(saved.contains(expected), "missing {expected:?} in\n{saved}");
}

#[then(expr = "the workspace settings file {string} still holds:")]
#[expect(
    clippy::expect_used,
    reason = "a Cucumber step fails by panicking, as a #[test] does"
)]
#[expect(
    clippy::needless_pass_by_value,
    clippy::needless_pass_by_ref_mut,
    reason = "Cucumber dictates step signatures: captured args arrive owned and the world as `&mut`"
)]
fn settings_unchanged(world: &mut TuiWorld, name: String, step: &Step) {
    let saved = fs::read_to_string(settings_file(world, &name)).expect("settings file");
    assert_eq!(saved, format!("{}\n", text(step)));
}
