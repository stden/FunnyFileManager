//! Tab-completion behaviour of the `:` command line.

use super::process_popup_input;
use super::tests::key;

use crate::models::{App, Overlay, Panel};

use crossterm::event::KeyCode;

use std::path::Path;

use tempfile::TempDir;

/// App whose active panel sits in `dir` with the command line holding `text`.
/// The panel directory matters: completion resolves paths relative to it.
fn app_with_command_line(dir: &Path, text: &str) -> App {
    let panel = Panel::new(dir.to_path_buf()).expect("panel");
    let mut app = App::with_panels(panel.clone(), panel);
    app.set_overlay(Overlay::CommandLine(text.into()));
    app
}

/// Two directories sharing a prefix make `cd al<Tab>` ambiguous: the line may
/// only advance to the longest common prefix, and the options open in the
/// dropdown menu so the user can see what is still open.
#[test]
fn tab_with_several_matches_inserts_the_common_prefix_and_opens_the_menu() {
    let temp = TempDir::new().expect("temp dir");
    std::fs::create_dir(temp.path().join("alpha_one")).expect("mkdir");
    std::fs::create_dir(temp.path().join("alpha_two")).expect("mkdir");
    let mut app = app_with_command_line(temp.path(), "cd al");

    assert!(process_popup_input(&mut app, key(KeyCode::Tab)));

    assert_eq!(app.get_command_line(), Some("cd alpha_"));
    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line should still be open");
    };
    let menu = state.menu.as_ref().expect("ambiguity opens the menu");
    // Candidate order comes from the completion engine (fish when it is
    // installed, the built-in directory scan otherwise), so assert on the
    // pieces rather than on their order.
    let texts: Vec<&str> = menu.candidates().iter().map(|c| c.text.as_str()).collect();
    assert!(texts.contains(&"alpha_one/"), "candidates: {texts:?}");
    assert!(texts.contains(&"alpha_two/"), "candidates: {texts:?}");
}

/// One candidate is not ambiguous: it is inserted whole and there is nothing to
/// choose from, so no menu opens and the status bar keeps whatever it showed.
#[test]
fn tab_with_a_single_match_completes_it_without_opening_the_menu() {
    let temp = TempDir::new().expect("temp dir");
    std::fs::create_dir(temp.path().join("solo")).expect("mkdir");
    let mut app = app_with_command_line(temp.path(), "cd so");
    let status_before = app.status_text();

    assert!(process_popup_input(&mut app, key(KeyCode::Tab)));

    assert_eq!(app.get_command_line(), Some("cd solo/"));
    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line should still be open");
    };
    assert!(state.menu.is_none(), "a lone candidate opens no menu");
    assert_eq!(app.status_text(), status_before);
}

/// No directory matches "zz", so Tab leaves the line untouched and opens no
/// menu. A directory argument avoids consulting executables on the host PATH.
#[test]
fn tab_with_no_match_leaves_the_line_alone_and_opens_no_menu() {
    let temp = TempDir::new().expect("temp dir");
    let mut app = app_with_command_line(temp.path(), "cd zz");
    let status_before = app.status_text();

    assert!(process_popup_input(&mut app, key(KeyCode::Tab)));

    assert_eq!(app.get_command_line(), Some("cd zz"));
    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line should still be open");
    };
    assert!(state.menu.is_none());
    assert_eq!(app.status_text(), status_before);
}
