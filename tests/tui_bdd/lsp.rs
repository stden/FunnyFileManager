//! Purpose: Steps for the Language servers dialog: a fake machine whose whole
//! `PATH` is the workspace's `bin`, and checks on what F2 wrote into the
//! workspace settings file.

use std::fs;

use cucumber::{given, then, when};
use funny_file_manager::test_support::{program_file, write_executable};
use ratatui::layout::Size;
use serde_norway::Value;

use super::{session::Session, settings::settings_file, TuiWorld};

/// The settings file every step here opens and reads.
const SETTINGS: &str = "settings.yaml";

#[given(expr = "the fake machine has the program {string}")]
#[expect(
    clippy::expect_used,
    reason = "a Cucumber step fails by panicking, as a #[test] does"
)]
#[expect(
    clippy::needless_pass_by_value,
    clippy::needless_pass_by_ref_mut,
    reason = "Cucumber dictates step signatures: captured args arrive owned and the world as `&mut`"
)]
fn program(world: &mut TuiWorld, name: String) {
    let bin = world.root().join("bin");
    fs::create_dir_all(&bin).expect("bin");
    write_executable(&bin.join(program_file(&name)), b"fixture").expect("program");
}

#[when("I open the language servers workspace")]
#[expect(
    clippy::expect_used,
    reason = "a Cucumber step fails by panicking, as a #[test] does"
)]
fn open_workspace(world: &mut TuiWorld) {
    let bin = world.root().join("bin");
    fs::create_dir_all(&bin).expect("bin");
    world.session = Some(Session::language_servers(
        (world.path("left", ""), world.path("right", "")),
        settings_file(world, SETTINGS),
        (world.root().join("lsp-cache"), bin),
        Size::new(120, 30),
    ));
}

#[then("the screen shows the settings file path")]
fn shows_path(world: &mut TuiWorld) {
    let path = settings_file(world, SETTINGS).display().to_string();
    let screen = world.ui().text();
    assert!(screen.contains(&path), "missing {path:?}\n{screen}");
}

/// A dot path into the settings file. A string is compared as written; any
/// other value as parsed YAML, so `false` and `1` mean themselves.
#[then(expr = "the workspace settings file {string} value {string} is {string}")]
#[expect(
    clippy::expect_used,
    reason = "a Cucumber step fails by panicking, as a #[test] does"
)]
#[expect(
    clippy::needless_pass_by_value,
    clippy::needless_pass_by_ref_mut,
    reason = "Cucumber dictates step signatures: captured args arrive owned and the world as `&mut`"
)]
fn value(world: &mut TuiWorld, name: String, path: String, expected: String) {
    let text = fs::read_to_string(settings_file(world, &name)).expect("settings file");
    let document: Value = serde_norway::from_str(&text).expect("settings YAML");
    let found = setting(&document, &path, &text);
    match found {
        Value::String(found) => assert_eq!(found, &expected, "{path}"),
        other => {
            let expected: Value = serde_norway::from_str(&expected).expect("expected YAML");
            assert_eq!(other, &expected, "{path}");
        }
    }
}

/// One dot-separated lookup. A missing key panics with the file text, which is
/// what the step shows when the settings document does not have that path.
#[expect(
    clippy::panic,
    reason = "a missing settings path fails the step the way a wrong assertion does"
)]
fn setting<'a>(document: &'a Value, path: &str, text: &str) -> &'a Value {
    path.split('.')
        .try_fold(document, |value, key| value.get(key))
        .unwrap_or_else(|| panic!("{path} is not in\n{text}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_settings_path_names_the_file() {
        let document = Value::String("x".to_owned());
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = setting(&document, "missing", "body");
        }));
        assert!(caught.is_err());
    }
}
