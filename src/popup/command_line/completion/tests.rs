use super::accept_ghost;
use crate::models::{App, CommandLine, Overlay, Panel};
use tempfile::TempDir;

#[test]
fn accept_ghost_does_nothing_when_no_ghost_is_shown() {
    let dir = TempDir::new().expect("temp");
    let panel = Panel::from_entries(dir.path().to_path_buf(), Vec::new());
    let mut app = App::with_panels(panel.clone(), panel);
    app.set_overlay(Overlay::CommandLine(CommandLine::from("git")));

    accept_ghost(&mut app);

    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line open");
    };
    assert_eq!(state.line.text(), "git");
    assert!(state.ghost.is_none());
}
