//! Gherkin steps only interact with the running UI and isolated fixture files.

// Cucumber dictates step signatures: captured args arrive owned and the world as `&mut`.
#![allow(clippy::needless_pass_by_value, clippy::needless_pass_by_ref_mut)]

use std::{fs, path::PathBuf};

use cucumber::{given, then, when};
use ratatui::layout::Size;
use tempfile::TempDir;

mod restart;
mod session;
mod settings;
#[then(expr = "the clipboard contains {string}")]
fn clipboard(world: &mut TuiWorld, text: String) {
    assert_eq!(world.ui().clipboard(), text);
}
#[when("I type the missing workspace directory")]
fn missing_directory(world: &mut TuiWorld) {
    let path = world.path("left", "missing-directory");
    world
        .session
        .as_mut()
        .expect("session")
        .type_text(&path.to_string_lossy());
}
#[cfg(feature = "keepass")]
mod vault;
use session::Session;

#[derive(cucumber::World, Debug, Default)]
pub struct TuiWorld {
    pub name: String,
    workspace: Option<TempDir>,
    session: Option<Session>,
}

impl TuiWorld {
    fn path(&self, side: &str, file: &str) -> PathBuf {
        assert!(matches!(side, "left" | "right"));
        self.workspace
            .as_ref()
            .expect("workspace")
            .path()
            .join(side)
            .join(file)
    }

    fn ui(&mut self) -> &mut Session {
        self.session.as_mut().expect("TUI is open")
    }

    pub fn finish(&mut self) {
        if let Some(mut session) = self.session.take() {
            session.stop();
            session.save_trace(&self.name).expect("save TUI transcript");
        }
    }
}

#[given("a TUI workspace")]
fn workspace(world: &mut TuiWorld) {
    let workspace = TempDir::new().expect("workspace");
    for side in ["left", "right"] {
        fs::create_dir(workspace.path().join(side)).expect("panel directory");
    }
    world.workspace = Some(workspace);
}

#[given(expr = "the {word} file {string} contains {string}")]
fn fixture(world: &mut TuiWorld, side: String, file: String, contents: String) {
    fs::write(world.path(&side, &file), contents).expect("fixture file");
}

#[when(expr = "I open the TUI at {int} by {int}")]
fn open(world: &mut TuiWorld, width: u16, height: u16) {
    world.session = Some(Session::start(
        world.path("left", ""),
        world.path("right", ""),
        Size::new(width, height),
    ));
}

#[when("I open the LSP inventory fixture")]
fn open_inventory(world: &mut TuiWorld) {
    world.session = Some(Session::inventory(
        world.path("left", ""),
        world.path("right", ""),
        Size::new(120, 30),
    ));
}

#[when(expr = "I press {string}")]
fn press(world: &mut TuiWorld, keys: String) {
    for key in keys.split_whitespace() {
        world.ui().press(key);
    }
}

#[when(expr = "I type {string}")]
fn type_text(world: &mut TuiWorld, text: String) {
    world.ui().type_text(&text);
}

#[when(expr = "the terminal pastes {string}")]
fn paste(world: &mut TuiWorld, text: String) {
    world.ui().paste(&text);
}

#[when(expr = "I resize the TUI to {int} by {int}")]
fn resize(world: &mut TuiWorld, width: u16, height: u16) {
    world.ui().resize(Size::new(width, height));
}

#[then(expr = "the screen contains {string}")]
fn contains(world: &mut TuiWorld, text: String) {
    let screen = world.ui().text();
    assert!(screen.contains(&text), "missing {text:?}\n{screen}");
}

#[then(expr = "the screen does not contain {string}")]
fn does_not_contain(world: &mut TuiWorld, text: String) {
    let screen = world.ui().text();
    assert!(!screen.contains(&text), "unexpected {text:?}\n{screen}");
}

#[then(expr = "the {word} panel shows {string}")]
fn panel_contains(world: &mut TuiWorld, side: String, text: String) {
    let screen = world.ui().panel_text(&side);
    let rows = usize::from(world.ui().size().height.saturating_sub(8));
    let found = screen.lines().skip(3).take(rows).any(|row| {
        row.contains(&text)
            || text.rsplit_once('.').is_some_and(|(stem, extension)| {
                row.split_once(stem)
                    .is_some_and(|(_, tail)| tail.trim_start().starts_with(extension))
            })
    });
    assert!(found, "missing {text:?} in {side}\n{screen}");
}

#[then(expr = "the {word} file {string} contains {string}")]
fn file_contents(world: &mut TuiWorld, side: String, file: String, expected: String) {
    assert_eq!(
        fs::read_to_string(world.path(&side, &file)).expect("file"),
        expected
    );
}

#[then(expr = "the {word} file {string} does not exist")]
fn absent(world: &mut TuiWorld, side: String, file: String) {
    assert!(!world.path(&side, &file).exists());
}

#[then(expr = "the {word} directory {string} exists")]
fn directory(world: &mut TuiWorld, side: String, name: String) {
    assert!(world.path(&side, &name).is_dir());
}

#[then(expr = "the screen size is {int} by {int}")]
fn size(world: &mut TuiWorld, width: u16, height: u16) {
    assert_eq!(world.ui().size(), Size::new(width, height));
}

#[then("the TUI exits successfully")]
fn exited(world: &mut TuiWorld) {
    world.ui().assert_exited();
}
