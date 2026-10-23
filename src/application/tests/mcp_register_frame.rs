//! Unit tests: Register in the Install MCP server dialog is drawn before it
//! runs, so a slow `claude` never freezes the screen without a word.

use super::*;
use crate::application::loop_state::run_after_frame;
use crate::models::Panel;
use crate::test_support::FakeMachine;

/// Review finding: registering ran on the key handler with no frame drawn,
/// so up to a minute of `claude` looked like a hang. The loop now draws the
/// frame whose status line says what is running, then registers.
#[test]
fn the_registering_frame_is_drawn_before_claude_runs() {
    let machine = FakeMachine::new().expect("fake machine");
    machine.install_clients().unwrap();
    machine.install_server().unwrap();
    let mut app = App::with_panels(
        Panel::new(machine.folder()).unwrap(),
        Panel::new(machine.folder()).unwrap(),
    );
    app.live.mcp_host = machine.host();
    crate::ops::mcp_install::open(&mut app);
    crate::ops::mcp_install::submit(&mut app);
    let mut terminal = Terminal::new(WritableTestBackend::new(80, 24)).unwrap();
    run_after_frame(&mut terminal, &mut app).unwrap();
    let screen = buffer_text(&terminal);
    assert!(
        screen.contains("Registering ffm-mcp: waiting for claude"),
        "{screen}"
    );
    assert!(screen.contains("Install MCP server"), "{screen}");
    assert!(matches!(app.overlay, Overlay::Viewer(_)), "then it ran");
    assert_eq!(machine.claude().calls().len(), 1);
}

#[test]
fn nothing_waiting_draws_nothing() {
    let directory = tempfile::TempDir::new().unwrap();
    let mut app = App::with_panels(
        Panel::new(directory.path().to_path_buf()).unwrap(),
        Panel::new(directory.path().to_path_buf()).unwrap(),
    );
    let mut terminal = Terminal::new(WritableTestBackend::new(80, 24)).unwrap();
    run_after_frame(&mut terminal, &mut app).unwrap();
    assert_eq!(buffer_text(&terminal).trim().len(), 0);
}
