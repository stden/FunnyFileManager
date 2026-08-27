//! Purpose: Exercises the readline key matrix of the `:` command-line overlay.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::models::{App, CommandLine, Overlay, Recall};
use crate::popup::process_popup_input;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}

fn ctrl(ch: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(ch), KeyModifiers::CONTROL)
}

fn alt(ch: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(ch), KeyModifiers::ALT)
}

/// An app with the prompt open on `text`, caret at the end.
fn prompt(text: &str) -> App {
    let mut app = App::new().expect("test app");
    app.set_overlay(Overlay::CommandLine(CommandLine::from(text)));
    app
}

/// The typed text and the caret's byte offset in it.
fn line(app: &App) -> (String, usize) {
    match &app.overlay {
        Overlay::CommandLine(state) => (state.line.text().to_owned(), state.line.cursor().byte()),
        other => panic!("command line closed: {:?}", std::mem::discriminant(other)),
    }
}

fn send(app: &mut App, events: &[KeyEvent]) {
    for event in events {
        assert!(
            process_popup_input(app, *event),
            "{:?} leaked out of the command line",
            event.code
        );
    }
}

// ─── cursor motion ────────────────────────────────────────────────────────

#[test]
fn left_and_right_move_the_caret_without_touching_the_text() {
    let mut app = prompt("cd /tmp");
    send(&mut app, &[key(KeyCode::Left), key(KeyCode::Left)]);
    assert_eq!(line(&app), ("cd /tmp".to_owned(), 5));

    send(&mut app, &[key(KeyCode::Right)]);
    assert_eq!(line(&app), ("cd /tmp".to_owned(), 6));
}

#[test]
fn typing_inserts_at_the_caret_rather_than_at_the_end() {
    let mut app = prompt("cd tmp");
    send(
        &mut app,
        &[
            key(KeyCode::Left),
            key(KeyCode::Left),
            key(KeyCode::Left),
            key(KeyCode::Char('/')),
        ],
    );
    assert_eq!(line(&app), ("cd /tmp".to_owned(), 4));
}

#[test]
fn home_and_end_jump_to_the_edges_of_the_line() {
    let mut app = prompt("cd /tmp");
    send(&mut app, &[key(KeyCode::Home)]);
    assert_eq!(line(&app).1, 0);
    send(&mut app, &[key(KeyCode::End)]);
    assert_eq!(line(&app).1, 7);
}

#[test]
fn ctrl_a_and_ctrl_e_are_home_and_end() {
    let mut app = prompt("cd /tmp");
    send(&mut app, &[ctrl('a')]);
    assert_eq!(line(&app).1, 0);
    send(&mut app, &[ctrl('e')]);
    assert_eq!(line(&app).1, 7);
}

#[test]
fn alt_b_and_alt_f_step_whole_words() {
    let mut app = prompt("cat /a/b.txt");
    send(&mut app, &[alt('b')]);
    assert_eq!(line(&app).1, 4);
    send(&mut app, &[alt('f')]);
    assert_eq!(line(&app).1, 12);
}

#[test]
fn ctrl_left_and_ctrl_right_step_whole_words_too() {
    let mut app = prompt("cat /a/b.txt");
    send(
        &mut app,
        &[KeyEvent::new(KeyCode::Left, KeyModifiers::CONTROL)],
    );
    assert_eq!(line(&app).1, 4);
    send(
        &mut app,
        &[KeyEvent::new(KeyCode::Right, KeyModifiers::CONTROL)],
    );
    assert_eq!(line(&app).1, 12);
}

// ─── deleting ─────────────────────────────────────────────────────────────

#[test]
fn backspace_and_delete_cut_on_either_side_of_the_caret() {
    let mut app = prompt("cdd /tmp");
    send(
        &mut app,
        &[ctrl('a'), key(KeyCode::Right), key(KeyCode::Right)],
    );
    send(&mut app, &[key(KeyCode::Backspace)]);
    assert_eq!(line(&app), ("cd /tmp".to_owned(), 1));

    send(&mut app, &[key(KeyCode::Delete)]);
    assert_eq!(line(&app), ("c /tmp".to_owned(), 1));
}

#[test]
fn ctrl_w_kills_the_word_before_the_caret_and_ctrl_y_puts_it_back() {
    let mut app = prompt("cat /a/b.txt");
    send(&mut app, &[ctrl('w')]);
    assert_eq!(line(&app), ("cat ".to_owned(), 4));

    send(&mut app, &[ctrl('y')]);
    assert_eq!(line(&app), ("cat /a/b.txt".to_owned(), 12));
}

#[test]
fn ctrl_k_kills_from_the_caret_to_the_end() {
    let mut app = prompt("cd /tmp");
    send(&mut app, &[alt('b'), ctrl('k')]);
    assert_eq!(line(&app), ("cd ".to_owned(), 3));
}

#[test]
fn ctrl_c_abandons_the_line_but_keeps_the_prompt_open() {
    let mut app = prompt("rm -rf /");
    send(&mut app, &[ctrl('c')]);
    assert_eq!(line(&app), (String::new(), 0));
    assert!(matches!(app.overlay, Overlay::CommandLine(_)));
}

#[test]
fn ctrl_d_closes_an_empty_prompt_and_forward_deletes_a_full_one() {
    let mut app = prompt("ls");
    send(&mut app, &[ctrl('a'), ctrl('d')]);
    assert_eq!(line(&app), ("s".to_owned(), 0));

    send(&mut app, &[ctrl('d')]);
    assert_eq!(line(&app), (String::new(), 0));

    send(&mut app, &[ctrl('d')]);
    assert!(matches!(app.overlay, Overlay::None), "empty Ctrl+D closes");
}

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

// ─── ghost text ───────────────────────────────────────────────────────────

#[test]
fn right_accepts_the_ghost_at_the_end_but_moves_the_caret_mid_line() {
    let mut app = prompt("");
    app.history.record("cd /very/long/path");

    send(
        &mut app,
        &[key(KeyCode::Char('c')), key(KeyCode::Char('d'))],
    );
    // At the end of the line, Right takes the suggestion.
    send(&mut app, &[key(KeyCode::Right)]);
    assert_eq!(line(&app).0, "cd /very/long/path");

    // Mid-line it is an ordinary cursor move, with no ghost to compete.
    send(&mut app, &[ctrl('a'), key(KeyCode::Right)]);
    assert_eq!(line(&app), ("cd /very/long/path".to_owned(), 1));
}

#[test]
fn the_ghost_is_suppressed_while_the_caret_sits_mid_line() {
    let mut app = prompt("");
    app.history.record("cd /very/long/path");
    send(
        &mut app,
        &[key(KeyCode::Char('c')), key(KeyCode::Char('d'))],
    );

    let ghost_at_end = match &app.overlay {
        Overlay::CommandLine(state) => state.ghost.clone(),
        _ => None,
    };
    assert!(ghost_at_end.is_some(), "a suggestion applies at the end");

    send(&mut app, &[key(KeyCode::Left)]);
    let ghost_mid_line = match &app.overlay {
        Overlay::CommandLine(state) => state.ghost.clone(),
        _ => None,
    };
    assert_eq!(
        ghost_mid_line, None,
        "a suggestion appended after mid-line text would describe a line \
         the user is not typing"
    );
}

// ─── completion at the caret ──────────────────────────────────────────────

#[test]
fn tab_completes_the_token_under_the_caret_and_keeps_the_tail() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    std::fs::create_dir(temp.path().join("Documents")).expect("mkdir");

    let mut app = prompt("cat Doc /etc/hosts");
    app.left_panel = crate::models::Panel::new(temp.path().to_path_buf()).expect("panel");
    app.active_panel = crate::models::PanelSide::Left;

    // Park the caret right after "Doc", with the second argument to its right.
    send(&mut app, &[ctrl('a')]);
    for _ in 0.."cat Doc".len() {
        send(&mut app, &[key(KeyCode::Right)]);
    }
    send(&mut app, &[key(KeyCode::Tab)]);

    let (text, cursor) = line(&app);
    assert_eq!(text, "cat Documents/ /etc/hosts");
    assert_eq!(
        &text[..cursor],
        "cat Documents/",
        "the caret stays at the end of what was just completed"
    );
}
