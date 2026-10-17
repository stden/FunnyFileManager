//! Unit tests: the `NetBox` dialogs — the plugin commands list, the input
//! prompt, the question, and the session dialog that never shows a password.

use tempfile::TempDir;

use super::netbox_sessions::{dialog_for, draw_overlay, find, terminal};
use crate::models::{
    InputPrompt, InputPurpose, Overlay, PluginCommands, Question, QuestionPurpose, RowMotion,
    SessionEdit,
};
use crate::test_support::buffer_text;
use crate::test_support::netbox::{app_in, name, session};
use crate::ui::palette;
use crate::ui::render_overlays;

#[test]
fn the_plugin_commands_list_draws_every_row_and_the_cursor() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    let mut list = PluginCommands::default();
    list.move_cursor(RowMotion::Next);
    app.set_overlay(Overlay::PluginCommands(list));
    let (screen, terminal) = draw_overlay(&mut app);
    for text in [
        " Plugin commands ",
        "MCP status",
        "NetBox: stored sessions",
        "NetBox: import from .netbox file…",
    ] {
        assert!(screen.contains(text), "{text:?} in\n{screen}");
    }
    let (x, y) = find(&screen, "NetBox: stored");
    assert_eq!(terminal.backend().buffer()[(x, y)].bg, palette::CURSOR_BG);
    let (x, y) = find(&screen, "MCP status");
    assert_eq!(terminal.backend().buffer()[(x, y)].bg, palette::MENU_BG);
    assert!(
        app.dialog_mouse.area.is_some(),
        "the frame is a drag handle"
    );
}

#[test]
fn the_input_prompt_draws_its_purpose_and_puts_the_caret_in_the_text() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    let mut prompt = InputPrompt::new(InputPurpose::ImportNetBox, "/srv/x/demo.netbox");
    prompt.input_mut().home();
    app.set_overlay(Overlay::InputPrompt(Box::new(prompt)));
    let mut terminal = terminal(80, 24);
    terminal
        .draw(|f| render_overlays(f, &mut app))
        .expect("draw");
    let screen = buffer_text(&terminal);
    for text in [
        "NetBox: import from .netbox file",
        "Read sessions from:",
        "/srv/x/demo.netbox",
        "OK",
        "Cancel",
    ] {
        assert!(screen.contains(text), "{text:?} in\n{screen}");
    }
    let (x, y) = find(&screen, "/srv/x/demo.netbox");
    let caret = terminal.get_cursor_position().expect("cursor");
    assert_eq!((caret.x, caret.y), (x, y), "the caret is at Home");
}

#[test]
fn the_question_draws_its_sentence_and_both_answers() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    app.set_overlay(Overlay::Question(Question::new(
        QuestionPurpose::DeleteSession(name("web")),
    )));
    let (screen, _) = draw_overlay(&mut app);
    for text in [
        "NetBox: delete session",
        "Delete the stored session web?",
        "Yes",
        "No",
    ] {
        assert!(screen.contains(text), "{text:?} in\n{screen}");
    }
}

const SESSION_DIALOG_TEXTS: [&str; 16] = [
    "New session",
    "Name",
    "Protocol",
    "SFTP",
    "FTPS",
    "Host",
    "example.test",
    "Port",
    "22",
    "User",
    "Login",
    "Agent",
    "Key file",
    "Directory",
    "Ok",
    "Cancel",
];

#[test]
fn the_session_dialog_draws_every_field_and_no_password() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    app.set_overlay(Overlay::SessionEdit(Box::new(dialog_for("example.test"))));
    let (screen, _) = draw_overlay(&mut app);
    for text in SESSION_DIALOG_TEXTS {
        assert!(screen.contains(text), "{text:?} in\n{screen}");
    }
    assert!(!screen.contains("Save password"), "{screen}");

    let stored = session("a.test", None);
    app.set_overlay(Overlay::SessionEdit(Box::new(SessionEdit::from_session(
        &stored,
    ))));
    let (screen, _) = draw_overlay(&mut app);
    assert!(screen.contains("Edit session"), "{screen}");
}
