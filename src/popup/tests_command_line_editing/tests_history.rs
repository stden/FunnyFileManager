//! Purpose: the command line's history — Up and Down recall, and Ctrl+R's reverse search.

use super::*;

// ─── history recall ───────────────────────────────────────────────────────

#[test]
fn up_recalls_the_previous_command_and_down_restores_what_was_typed() {
    let mut app = prompt("");
    app.history.record("cd /one");
    app.history.record("cd /two");

    // Half-typed text is stashed on the first Up, not lost.
    send(
        &mut app,
        &[key(KeyCode::Char('l')), key(KeyCode::Char('s'))],
    );
    send(&mut app, &[key(KeyCode::Up)]);
    assert_eq!(line(&app).0, "cd /two");
    send(&mut app, &[key(KeyCode::Up)]);
    assert_eq!(line(&app).0, "cd /one");

    // Holding Up past the oldest entry stays put rather than blanking.
    send(&mut app, &[key(KeyCode::Up), key(KeyCode::Up)]);
    assert_eq!(line(&app).0, "cd /one");

    send(&mut app, &[key(KeyCode::Down)]);
    assert_eq!(line(&app).0, "cd /two");
    send(&mut app, &[key(KeyCode::Down)]);
    assert_eq!(line(&app).0, "ls", "the stashed line comes back");
    assert!(matches!(
        app.overlay,
        Overlay::CommandLine(CommandLine {
            recall: Recall::Idle,
            ..
        })
    ));
}

#[test]
fn down_on_an_untouched_prompt_does_nothing() {
    let mut app = prompt("ls");
    app.history.record("cd /one");
    send(&mut app, &[key(KeyCode::Down)]);
    assert_eq!(line(&app).0, "ls");
}

#[test]
fn a_recalled_command_can_be_edited_and_executed() {
    let mut app = prompt("");
    app.history.record("cd /one");

    send(&mut app, &[key(KeyCode::Up)]);
    send(
        &mut app,
        &[key(KeyCode::Backspace), key(KeyCode::Char('2'))],
    );
    assert_eq!(line(&app).0, "cd /on2");
}

// ─── reverse search ───────────────────────────────────────────────────────

#[test]
fn ctrl_r_searches_backwards_and_enter_accepts_the_hit() {
    let mut app = prompt("");
    app.history.record("grep needle /var/log");
    app.history.record("cd /tmp");

    send(&mut app, &[ctrl('r')]);
    assert!(matches!(
        app.overlay,
        Overlay::CommandLine(CommandLine {
            recall: Recall::Searching { .. },
            ..
        })
    ));

    send(
        &mut app,
        &[key(KeyCode::Char('n')), key(KeyCode::Char('e'))],
    );
    assert_eq!(line(&app).0, "grep needle /var/log");

    // Enter accepts into the prompt without executing, so the recalled command
    // can be read before it runs.
    send(&mut app, &[key(KeyCode::Enter)]);
    assert_eq!(line(&app).0, "grep needle /var/log");
    assert!(matches!(
        app.overlay,
        Overlay::CommandLine(CommandLine {
            recall: Recall::Idle,
            ..
        })
    ));
}

#[test]
fn a_repeated_ctrl_r_steps_to_the_next_older_match() {
    let mut app = prompt("");
    app.history.record("cd /alpha");
    app.history.record("cd /beta");

    send(&mut app, &[ctrl('r'), key(KeyCode::Char('c'))]);
    assert_eq!(line(&app).0, "cd /beta");
    send(&mut app, &[ctrl('r')]);
    assert_eq!(line(&app).0, "cd /alpha");
}

#[test]
fn esc_abandons_a_search_and_puts_the_typed_line_back() {
    let mut app = prompt("");
    app.history.record("cd /alpha");
    send(
        &mut app,
        &[key(KeyCode::Char('l')), key(KeyCode::Char('s'))],
    );

    send(&mut app, &[ctrl('r'), key(KeyCode::Char('c'))]);
    assert_eq!(line(&app).0, "cd /alpha");

    send(&mut app, &[key(KeyCode::Esc)]);
    assert_eq!(line(&app).0, "ls");
    assert!(
        matches!(app.overlay, Overlay::CommandLine(_)),
        "Esc leaves the search, not the prompt"
    );
}

#[test]
fn a_search_that_matches_nothing_keeps_the_last_hit_on_screen() {
    let mut app = prompt("");
    app.history.record("cd /alpha");

    send(&mut app, &[ctrl('r'), key(KeyCode::Char('c'))]);
    assert_eq!(line(&app).0, "cd /alpha");
    // 'z' makes the query unmatchable; bash keeps showing the last match.
    send(&mut app, &[key(KeyCode::Char('z'))]);
    assert_eq!(line(&app).0, "cd /alpha");
    // Backspacing the bad character finds it again.
    send(&mut app, &[key(KeyCode::Backspace)]);
    assert_eq!(line(&app).0, "cd /alpha");
}
