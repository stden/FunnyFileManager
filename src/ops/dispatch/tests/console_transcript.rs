//! Purpose: What typed commands leave in the console transcript behind the panels.

use super::*;

// ─── console transcript ───────────────────────────────────────────────

/// The transcript as plain text, oldest first.
fn console_text(app: &App) -> Vec<String> {
    app.console
        .visible(usize::MAX)
        .into_iter()
        .map(|line| line.text().to_owned())
        .collect()
}

#[test]
fn command_output_lands_in_the_console_behind_the_panels() {
    let temp = TempDir::new().expect("temp dir");
    let mut app = make_app(&temp);

    execute_command(&mut app, r"printf 'alpha\nbeta\n'").expect("shell command");

    let transcript = console_text(&app);
    assert_eq!(
        transcript[0],
        format!(r"{}> printf 'alpha\nbeta\n'", temp.path().display()),
        "the command is echoed behind its prompt, as Far prints it"
    );
    assert_eq!(&transcript[1..], ["alpha".to_owned(), "beta".to_owned()]);
    // No window pops up over the panels — that is the whole point.
    assert!(matches!(app.overlay, Overlay::None));
    // The status bar still summarises, so the common case costs no keys.
    assert_eq!(app.status_text(), "beta");
}

#[test]
fn stderr_is_recorded_after_stdout_and_tagged_as_an_error() {
    let temp = TempDir::new().expect("temp dir");
    let mut app = make_app(&temp);

    // `cmd.exe` chains with `&`, not `;` — it passes `;` on as an argument —
    // and `>&2` there redirects the *whole* line, so the redirect has to lead
    // and sit tight against the word to avoid echoing the space before it.
    #[cfg(windows)]
    let cmd = "echo out&1>&2 echo err";
    #[cfg(not(windows))]
    let cmd = r"printf 'out\n'; printf 'err\n' >&2";

    execute_command(&mut app, cmd).expect("shell command");

    let lines = app.console.visible(usize::MAX);
    assert!(matches!(lines[1], crate::console::ConsoleLine::Output(_)));
    assert!(matches!(lines[2], crate::console::ConsoleLine::Error(_)));
    assert_eq!(lines[1].text(), "out");
    assert_eq!(lines[2].text(), "err");
}

#[test]
fn built_in_commands_are_echoed_too() {
    // A `cd` missing from the transcript makes the shell output around it
    // impossible to place.
    let temp = TempDir::new().expect("temp dir");
    let mut app = make_app(&temp);

    execute_command(&mut app, "pwd").expect("built-in");

    assert_eq!(
        console_text(&app),
        [format!("{}> pwd", temp.path().display())]
    );
}

#[test]
fn a_command_that_cannot_be_launched_leaves_its_failure_in_the_console() {
    let temp = TempDir::new().expect("temp dir");
    let mut app = make_app(&temp);
    // The panel directory is gone, so the shell cannot even be spawned in it.
    app.get_active_panel().path = temp.path().join("does-not-exist");

    let result = execute_command(&mut app, "true");

    assert!(result.is_err());
    let transcript = console_text(&app);
    assert!(
        transcript
            .last()
            .is_some_and(|line| line.contains("cannot run 'true'")),
        "an echoed command with nothing under it reads as a silent success: {transcript:?}"
    );
}

#[test]
fn the_status_bar_agrees_with_the_transcript_about_colour_codes() {
    // Otherwise the summary reads `[31mred[0m` while the console two
    // keystrokes away shows `red`.
    let temp = TempDir::new().expect("temp dir");
    let mut app = make_app(&temp);

    execute_command(&mut app, r"printf '\033[31mred\033[0m\n'").expect("shell command");

    assert_eq!(app.status_text(), "red");
    assert_eq!(console_text(&app).last().map(String::as_str), Some("red"));
}

#[test]
fn an_empty_shell_command_is_rejected() {
    let temp = TempDir::new().expect("temp dir");
    let mut app = make_app(&temp);
    assert!(execute_command(&mut app, "   ").is_err());
}
