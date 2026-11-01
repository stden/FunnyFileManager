//! Purpose: What choosing a row of a menu dialog opens.

use super::*;

#[test]
fn choosing_a_command_opens_the_command_line_holding_that_command() {
    let (_temp, mut app) = fixture();
    choose(&mut app, ChoiceTarget::Command("mkdir docs".to_owned())).unwrap();
    assert!(
        matches!(app.overlay, Overlay::CommandLine(_)),
        "a command row opens the command line"
    );
    assert_eq!(
        app.get_command_line(),
        Some("mkdir docs"),
        "the command line carries the chosen command"
    );
}
