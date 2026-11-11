//! Purpose: When steps: opening the TUI, then pressing, typing, pasting and resizing.

use super::*;

#[when("I type the missing workspace directory")]
fn missing_directory(world: &mut TuiWorld) {
    let path = world.path("left", "missing-directory");
    world.ui().type_text(&path.to_string_lossy());
}

#[when(expr = "I open the TUI at {int} by {int}")]
fn open(world: &mut TuiWorld, width: u16, height: u16) {
    let (left, right, size) = (
        world.path("left", ""),
        world.path("right", ""),
        Size::new(width, height),
    );
    world.session = Some(match &world.mcp {
        Some(machine) => Session::start_on(left, right, size, machine.host()),
        None => Session::start(left, right, size),
    });
}

#[when(expr = "I press {string}")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Cucumber dictates step signatures: captured args arrive owned"
)]
fn press(world: &mut TuiWorld, keys: String) {
    for key in keys.split_whitespace() {
        world.ui().press(key);
    }
}

#[when(expr = "I type {string}")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Cucumber dictates step signatures: captured args arrive owned"
)]
fn type_text(world: &mut TuiWorld, text: String) {
    world.ui().type_text(&text);
}

#[when(expr = "the terminal pastes {string}")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Cucumber dictates step signatures: captured args arrive owned"
)]
fn paste(world: &mut TuiWorld, text: String) {
    world.ui().paste(&text);
}

#[when(expr = "I resize the TUI to {int} by {int}")]
fn resize(world: &mut TuiWorld, width: u16, height: u16) {
    world.ui().resize(Size::new(width, height));
}
