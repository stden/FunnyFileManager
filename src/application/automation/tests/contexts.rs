//! Purpose: Nested prompt routing and panel subcontexts cannot consume the wrong named action.

use super::*;
use crate::models::{Candidate, CandidateKind, CommandLine, Menu, Modal, Recall, TaskList};

fn completed_line() -> CommandLine {
    CommandLine {
        menu: Menu::new(vec![Candidate {
            text: "pwd".into(),
            kind: CandidateKind::Word,
            hint: None,
        }]),
        ..CommandLine::from("pw")
    }
}

#[test]
fn completion_and_reverse_search_own_their_nested_contexts() {
    let mut fixture = Fixture::new();
    fixture.app.overlay = Overlay::CommandLine(completed_line());
    assert_eq!(
        current_context(&fixture.app),
        Some(crate::keymap::Context::CommandLineCompletion)
    );
    let original = fixture.app.keymap.clone();
    fixture.run("command_line_completion", "accept").unwrap();
    let Overlay::CommandLine(line) = &fixture.app.overlay else {
        panic!("command line closed")
    };
    assert_eq!(line.text(), "pwd ");
    assert!(line.menu.is_none());
    assert_eq!(fixture.app.keymap, original);
    fixture.run("command_line", "history_search").unwrap();
    assert_eq!(
        current_context(&fixture.app),
        Some(crate::keymap::Context::CommandLineSearch)
    );
    assert!(fixture.run("command_line", "submit").is_err());
    fixture.run("command_line_search", "cancel").unwrap();
    assert_eq!(
        current_context(&fixture.app),
        Some(crate::keymap::Context::CommandLine)
    );
}

#[test]
fn line_editing_is_not_intercepted_by_a_custom_completion_chord() {
    let mut fixture = Fixture::new();
    fixture.app.overlay = Overlay::CommandLine(completed_line());
    fixture.app.keymap.command_line_completion.bind(
        crate::action::command_line::completion::CompletionCommand::Accept,
        "F24".parse().unwrap(),
    );
    let original = fixture.app.keymap.clone();
    fixture.run("command_line", "backspace").unwrap();
    let Overlay::CommandLine(line) = &fixture.app.overlay else {
        panic!("command line closed")
    };
    assert_eq!(line.text(), "p");
    assert_eq!(fixture.app.keymap, original);
}

#[test]
fn editor_search_and_confirmations_refuse_document_actions() {
    let mut fixture = Fixture::new();
    fixture.editor();
    fixture.run("editor", "search").unwrap();
    assert_eq!(current_context(&fixture.app), None);
    assert!(fixture.run("editor", "save").is_err());
    let editor = fixture.app.get_editor_mut().unwrap();
    editor.search.prompt = None;
    editor.state = crate::models::EditState::ConfirmingExit(crate::models::ExitChoice::Save);
    assert_eq!(current_context(&fixture.app), None);
    assert!(fixture.run("editor", "newline").is_err());
    assert_eq!(
        fs::read_to_string(fixture.root.path().join("left/file.txt")).unwrap(),
        "alpha"
    );
}

#[test]
fn task_search_is_distinct_and_other_task_dialogs_refuse_table_actions() {
    let mut fixture = Fixture::new();
    let mut tasks = TaskList::default();
    let mut search = crate::models::SearchBar::default();
    search.line_mut().insert_str("missing-process");
    tasks.set_modal(Modal::Search(search));
    fixture.app.overlay = Overlay::Tasks(Box::new(tasks));
    assert_eq!(
        current_context(&fixture.app),
        Some(crate::keymap::Context::TaskListSearch)
    );
    fixture.run("task_list_search", "find_next").unwrap();
    let Modal::Search(search) = fixture.app.task_list_mut().unwrap().modal() else {
        panic!("search closed")
    };
    assert_eq!(search.found(), crate::models::Found::NoMatch);
    assert!(fixture.run("task_list", "close").is_err());
    fixture
        .app
        .task_list_mut()
        .unwrap()
        .set_modal(Modal::Help(crate::scroll::ScrollTop::TOP));
    assert_eq!(current_context(&fixture.app), None);
    fixture.app.task_list_mut().unwrap().close_modal();
    fixture.run("task_list", "close").unwrap();
    assert!(matches!(fixture.app.overlay, Overlay::None));
}

#[test]
fn explicit_search_state_has_priority_over_an_existing_completion_menu() {
    let mut fixture = Fixture::new();
    let mut line = completed_line();
    line.recall = Recall::Searching {
        query: String::new(),
        hit: None,
        stash: "pw".into(),
    };
    fixture.app.overlay = Overlay::CommandLine(line);
    assert_eq!(
        current_context(&fixture.app),
        Some(crate::keymap::Context::CommandLineSearch)
    );
    assert!(fixture.run("command_line_completion", "accept").is_err());
}

#[test]
fn named_task_browse_commands_ignore_chords_and_use_the_real_tab_and_close_effects() {
    let mut fixture = Fixture::new();
    fixture.app.overlay = Overlay::Tasks(Box::default());
    fixture.app.keymap.task_list = crate::action::Bindings::default();
    fixture.app.keymap.task_list.bind(
        crate::action::tasks::TaskCommand::Help,
        "F24".parse().unwrap(),
    );
    let original = fixture.app.keymap.clone();
    fixture.run("task_list", "toggle_tab").unwrap();
    assert_eq!(
        fixture.app.task_list_mut().unwrap().tab(),
        crate::models::TaskTab::Network
    );
    fixture.run("task_list", "close").unwrap();
    assert!(matches!(fixture.app.overlay, Overlay::None));
    assert_eq!(fixture.app.keymap, original);
}

#[test]
fn the_typed_task_entry_point_refuses_commands_behind_a_nested_dialog() {
    let mut fixture = Fixture::new();
    let mut tasks = TaskList::default();
    tasks.set_modal(Modal::Help(crate::scroll::ScrollTop::TOP));
    fixture.app.overlay = Overlay::Tasks(Box::new(tasks));
    assert!(!crate::popup::tasks::invoke(
        &mut fixture.app,
        crate::action::tasks::TaskCommand::Close
    ));
    assert!(matches!(
        fixture.app.task_list_mut().unwrap().modal(),
        Modal::Help(_)
    ));
}
