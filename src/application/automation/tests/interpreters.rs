//! Shared typed interpreters retain keyboard behavior and modal boundaries.

use super::*;
use crate::action::{editor::EditorCommand, viewer::ViewerScroll};
use crate::models::{CommandLine, Viewer, Viewport};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[test]
fn typed_editor_commands_clear_notices_and_respect_nested_dialogs() {
    let mut fixture = Fixture::new();
    fixture.editor();
    fixture.app.get_editor_mut().unwrap().notice = Some("Saved".into());
    popup::editor::apply_editor_command(&mut fixture.app, EditorCommand::Newline);
    assert_eq!(fixture.app.get_editor().unwrap().content(), "\nalpha");
    assert!(fixture.app.get_editor().unwrap().notice.is_none());
    fixture.run("editor", "search").unwrap();
    popup::editor::apply_editor_command(&mut fixture.app, EditorCommand::Insert('x'));
    assert_eq!(fixture.app.get_editor().unwrap().content(), "\nalpha");
    assert!(fixture.app.get_editor().unwrap().search.prompt.is_some());
}

#[test]
fn typed_viewer_commands_use_the_keyboard_page_size_and_preserve_bindings() {
    let mut fixture = Fixture::new();
    fixture.app.set_viewport(Viewport::new(10));
    let viewer = Viewer::report("Report", (0..100).map(|row| row.to_string()).collect());
    fixture.app.set_overlay(Overlay::Viewer(viewer.clone()));
    let original = fixture.app.keymap.clone();
    popup::viewer::apply(
        &mut fixture.app,
        ViewerCommand::Scroll(ViewerScroll::PageDown),
    );
    let typed_scroll = fixture.app.get_viewer().unwrap().scroll;
    fixture.app.set_overlay(Overlay::Viewer(viewer));
    popup::process_popup_input(
        &mut fixture.app,
        KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE),
    );
    assert_ne!(typed_scroll, crate::scroll::ScrollTop::TOP);
    assert_eq!(fixture.app.get_viewer().unwrap().scroll, typed_scroll);
    assert_eq!(fixture.app.keymap, original);
}

#[test]
fn typed_line_editing_does_not_execute_behind_reverse_search() {
    let mut fixture = Fixture::new();
    fixture.app.overlay = Overlay::CommandLine(CommandLine::from("draft"));
    fixture.run("command_line", "history_search").unwrap();
    popup::command_line::apply(
        &mut fixture.app,
        crate::action::command_line::LineCommand::Cancel,
    );
    let Overlay::CommandLine(line) = &fixture.app.overlay else {
        panic!("search lost its command line")
    };
    assert!(matches!(
        line.recall,
        crate::models::Recall::Searching { .. }
    ));
    assert_eq!(line.text(), "draft");
}

#[test]
fn typed_completion_commands_leave_reverse_search_in_control() {
    let mut fixture = Fixture::new();
    let mut line = contexts::completed_line();
    line.recall = crate::models::Recall::Searching {
        query: String::new(),
        hit: None,
        stash: "pw".into(),
    };
    fixture.app.overlay = Overlay::CommandLine(line);
    let close = crate::action::command_line::completion::CompletionCommand::Close;
    popup::command_line::apply_completion(&mut fixture.app, close);
    let Overlay::CommandLine(line) = &fixture.app.overlay else {
        panic!("command line closed")
    };
    assert!(line.menu.is_some());
    fixture.run("command_line_search", "cancel").unwrap();
    popup::command_line::apply_completion(&mut fixture.app, close);
    let Overlay::CommandLine(line) = &fixture.app.overlay else {
        panic!("command line closed")
    };
    assert!(line.menu.is_none());
    assert_eq!(line.text(), "pw");
}
