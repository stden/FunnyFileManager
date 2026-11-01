//! Unit tests: the `NetBox` dialogs cell for cell — the plugin list's width,
//! the focus colour on buttons and answers, the session dialog's choices
//! and buttons, and the caret at the end of the focused field.

use ratatui::layout::Rect;
use ratatui::Terminal;
use tempfile::TempDir;

use crate::models::{
    App, AuthKind, InputPrompt, InputPurpose, Overlay, PluginCommand, PluginCommands, Protocol,
    Question, QuestionPurpose, SessionEdit,
};
use crate::test_support::netbox::{app_in, name};
use crate::test_support::{buffer_text, WritableTestBackend};
use crate::ui::{palette, render_overlays};

fn draw_overlay(app: &mut App) -> (String, Terminal<WritableTestBackend>) {
    let mut terminal = Terminal::new(WritableTestBackend::new(80, 24)).expect("terminal");
    terminal.draw(|f| render_overlays(f, app)).expect("draw");
    (buffer_text(&terminal), terminal)
}

/// The cell where `text` starts on `screen`.
fn at(screen: &str, text: &str) -> (u16, u16) {
    screen
        .lines()
        .enumerate()
        .find_map(|(row, line)| {
            let (before, _) = line.split_once(text)?;
            let column = u16::try_from(before.chars().count()).expect("fits");
            Some((column, u16::try_from(row).expect("fits")))
        })
        .unwrap_or_else(|| panic!("{text:?} on\n{screen}"))
}

#[test]
fn the_plugin_list_is_as_wide_as_its_longest_row_plus_padding_and_frame() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    app.set_overlay(Overlay::PluginCommands(PluginCommands::default()));
    drop(draw_overlay(&mut app));
    let longest = PluginCommand::ImportNetBox.label().chars().count();
    let expected = super::drives::centered(
        Rect::new(0, 0, 80, 24),
        1 + longest + 1 + 2,
        PluginCommand::ALL.len() + 2,
    );
    assert_eq!(app.dialog_mouse.area, Some(expected));
}

#[test]
fn the_focused_button_carries_the_focus_colour() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    let mut prompt = InputPrompt::new(InputPurpose::ImportNetBox, "");
    app.set_overlay(Overlay::InputPrompt(Box::new(prompt.clone())));
    let (screen, terminal) = draw_overlay(&mut app);
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[at(&screen, "OK")].bg, palette::DIALOG_FOCUS_BG);
    assert_ne!(buffer[at(&screen, "Cancel")].bg, palette::DIALOG_FOCUS_BG);

    prompt.toggle_button();
    app.set_overlay(Overlay::InputPrompt(Box::new(prompt)));
    let (screen, terminal) = draw_overlay(&mut app);
    let buffer = terminal.backend().buffer();
    assert_ne!(buffer[at(&screen, "OK")].bg, palette::DIALOG_FOCUS_BG);
    assert_eq!(buffer[at(&screen, "Cancel")].bg, palette::DIALOG_FOCUS_BG);
}

#[test]
fn the_questions_answer_under_the_cursor_carries_the_focus_colour() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    let mut question = Question::new(QuestionPurpose::DeleteSession(name("web")));
    app.set_overlay(Overlay::Question(question.clone()));
    let (screen, terminal) = draw_overlay(&mut app);
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[at(&screen, "Yes")].bg, palette::DIALOG_FOCUS_BG);
    assert_ne!(buffer[at(&screen, "No ")].bg, palette::DIALOG_FOCUS_BG);

    question.toggle();
    app.set_overlay(Overlay::Question(question));
    let (screen, terminal) = draw_overlay(&mut app);
    let buffer = terminal.backend().buffer();
    assert_ne!(buffer[at(&screen, "Yes")].bg, palette::DIALOG_FOCUS_BG);
    assert_eq!(buffer[at(&screen, "No ")].bg, palette::DIALOG_FOCUS_BG);
}

#[test]
fn the_session_dialog_marks_the_chosen_protocol_and_login() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    let mut dialog = SessionEdit::create();
    dialog.protocol = Protocol::Ftps;
    dialog.auth = AuthKind::Agent;
    app.set_overlay(Overlay::SessionEdit(Box::new(dialog)));
    let (screen, _) = draw_overlay(&mut app);
    for text in ["(•) FTPS", "( ) SFTP", "(•) Agent", "( ) Password"] {
        assert!(screen.contains(text), "{text:?} in\n{screen}");
    }
}

#[test]
fn the_session_dialogs_buttons_take_the_focus_in_turn() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    let mut dialog = SessionEdit::create();
    for _ in 0..7 {
        dialog.tab(true);
    }
    assert_eq!(dialog.field(), crate::models::SessionField::Ok);
    app.set_overlay(Overlay::SessionEdit(Box::new(dialog.clone())));
    let (screen, terminal) = draw_overlay(&mut app);
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[at(&screen, "Ok ")].bg, palette::DIALOG_FOCUS_BG);
    assert_ne!(buffer[at(&screen, "Cancel")].bg, palette::DIALOG_FOCUS_BG);

    dialog.tab(true);
    app.set_overlay(Overlay::SessionEdit(Box::new(dialog)));
    let (screen, terminal) = draw_overlay(&mut app);
    let buffer = terminal.backend().buffer();
    assert_ne!(buffer[at(&screen, "Ok ")].bg, palette::DIALOG_FOCUS_BG);
    assert_eq!(buffer[at(&screen, "Cancel")].bg, palette::DIALOG_FOCUS_BG);
}

/// Typing lands in the focused field, so the caret stands at its end.
#[test]
fn the_caret_stands_at_the_end_of_the_focused_field() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    let mut dialog = SessionEdit::create();
    dialog.name = "web".to_owned();
    dialog.tab(true);
    dialog.tab(true);
    dialog.host = "example.test".to_owned();
    assert_eq!(dialog.field(), crate::models::SessionField::Host);
    app.set_overlay(Overlay::SessionEdit(Box::new(dialog)));
    let (screen, mut terminal) = draw_overlay(&mut app);
    let (x, y) = at(&screen, "example.test");
    let caret = terminal.get_cursor_position().expect("cursor");
    assert_eq!((caret.x, caret.y), (x + 12, y));
}
