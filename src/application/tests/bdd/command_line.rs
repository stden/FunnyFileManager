//! Check the command line and submit input that has no executable action.

use cucumber::{then, when};

use super::BehaviorWorld;
use crate::command::Command;

#[then(expr = "the command line equals {string}")]
fn command_line_equals(world: &mut BehaviorWorld, expected: String) {
    assert_eq!(world.case().app.get_command_line(), Some(expected.as_str()));
}

#[when("I submit the invalid built-in command")]
fn submit_invalid_command(world: &mut BehaviorWorld) {
    let case = world.case();
    let line = case.app.get_command_line().expect("open command line");
    // Fail before Enter if the input could reach a real command.
    assert!(matches!(Command::parse(line), Command::Invalid { .. }));
    case.press("Enter");
}
