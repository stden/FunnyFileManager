//! Purpose: Enter on an executable runs it; a plain file is left alone.

use super::*;

// ─── Enter on an executable runs it ─────────────────────────────────────────

#[cfg(windows)]
#[test]
fn enter_runs_a_cmd_file_from_the_active_panel() {
    let (mut app, mut terminal, dir) = app_and_terminal(&[]);
    fs::write(
        dir.path().join("my script.cmd"),
        "@echo off\r\necho entered>entered.txt\r\n",
    )
    .expect("script");
    app.left_panel.load_entries().expect("reload");
    select_named(&mut app.left_panel, "my script.cmd");
    let _session = ProgramTerminalGuard::enter();
    press(&mut app, &mut terminal, KeyCode::Enter);
    assert_eq!(
        fs::read_to_string(dir.path().join("entered.txt"))
            .expect("Enter launched cmd")
            .trim(),
        "entered"
    );
}

#[cfg(windows)]
struct ProgramTerminalGuard;

#[cfg(windows)]
impl ProgramTerminalGuard {
    fn enter() -> Self {
        let _ = crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture);
        Self
    }
}

#[cfg(windows)]
impl Drop for ProgramTerminalGuard {
    fn drop(&mut self) {
        let _ = crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture);
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

#[cfg(unix)]
#[test]
fn enter_on_an_executable_runs_it_rather_than_doing_nothing() {
    use std::os::unix::fs::PermissionsExt;

    let (mut app, mut terminal, dir) = app_and_terminal(&[]);
    let script = dir.path().join("greet.sh");
    fs::write(&script, "#!/bin/sh\necho hello-from-panel > launched.txt\n").expect("test setup");
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).expect("test setup");
    app.left_panel.load_entries().expect("reload");
    select_named(&mut app.left_panel, "greet.sh");

    press(&mut app, &mut terminal, KeyCode::Enter);

    assert_eq!(
        fs::read_to_string(dir.path().join("launched.txt")).expect("program ran"),
        "hello-from-panel\n"
    );
}

/// …and Enter on an ordinary file still enters nothing, so the branch above
/// cannot be reached by anything that merely looks like a program.
#[test]
fn enter_on_a_plain_file_does_not_try_to_run_it() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&["notes.txt"]);
    select_named(&mut app.left_panel, "notes.txt");
    let before = app.get_active_panel_ref().path.clone();

    press(&mut app, &mut terminal, KeyCode::Enter);

    assert_eq!(app.get_active_panel_ref().path, before);
    assert!(console_text(&app).is_empty(), "nothing was run");
}
