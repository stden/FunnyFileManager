use super::search_from;
use crate::history::RingIndex;
use crate::models::{App, CommandLine, Overlay, Panel};
use tempfile::TempDir;

#[test]
fn search_from_is_inert_when_command_line_is_idle() {
    let dir = TempDir::new().expect("temp");
    let panel = Panel::from_entries(dir.path().to_path_buf(), Vec::new());
    let mut app = App::with_panels(panel.clone(), panel);
    app.history.record("git status");
    app.set_overlay(Overlay::CommandLine(CommandLine::from("abc")));

    search_from(&mut app, RingIndex::NEWEST);

    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line open");
    };
    assert_eq!(state.line.text(), "abc");
}
