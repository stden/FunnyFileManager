//! Purpose: Exercises the readline key matrix of the `:` command-line overlay.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::models::{App, CommandLine, Overlay, Recall};
use crate::popup::process_popup_input;

mod tests_ghost_and_completion;
mod tests_history;

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
