//! Purpose: When the panels come back after a program has run.

use super::*;

/// What the program printed sits on the terminal's own screen; the panels
/// would cover it at once, so they wait for a key, as the output's last line.
#[test]
fn the_panels_come_back_only_after_a_key_once_the_program_has_run() {
    let (_root, mut app, mut terminal) = fixture();
    app.live.program_pause = ProgramPause::UntilKey;
    let mut ops = PauseOps::default();
    launch_with(&mut terminal, &mut app, &mut ops, |_, _| {
        Ok(ExitStatus::default())
    })
    .expect("terminal restored");
    assert_eq!(
        ops.prompts,
        [format!(
            "[{}] Press any key to return to FFM",
            ExitStatus::default()
        )]
    );
}

/// A program that never started printed nothing worth pausing for; the
/// status bar has the reason.
#[test]
fn a_launch_failure_returns_to_the_panels_at_once() {
    let (_root, mut app, mut terminal) = fixture();
    let mut ops = PauseOps::default();
    launch_with(&mut terminal, &mut app, &mut ops, |_, _| {
        Err(io::Error::other("exec format error"))
    })
    .expect("terminal restored");
    assert_eq!(ops.prompts.len(), 0);
    assert_eq!(
        transcript(&app).last().map(String::as_str),
        Some(app.status_text().as_str())
    );
}

/// Without a person at the terminal (every test's `App::new()`, a replay)
/// nothing could answer the pause, so the panels return at once. On Windows
/// the pause used to wait for a console key that CI never sends.
#[test]
fn without_a_pause_the_panels_return_as_soon_as_the_program_ends() {
    let (_root, mut app, mut terminal) = fixture();
    assert_eq!(app.live.program_pause, ProgramPause::None);
    let mut ops = PauseOps::default();
    launch_with(&mut terminal, &mut app, &mut ops, |_, _| {
        Ok(ExitStatus::default())
    })
    .expect("terminal restored");
    assert_eq!(ops.prompts.len(), 0, "{:?}", ops.prompts);
}
