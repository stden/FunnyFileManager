//! Purpose: A server with no command shows the placeholder, and a real command
//! is shown as typed.

use super::*;

#[test]
fn a_blank_command_shows_the_placeholder() {
    let pieces = command_pieces("   ");
    assert_eq!(pieces.len(), 1);
    assert_eq!(pieces[0].0, NOTHING);
}

#[test]
fn a_command_is_shown_as_typed() {
    let pieces = command_pieces("bat --plain");
    assert_eq!(pieces[0].0, "bat --plain");
}
