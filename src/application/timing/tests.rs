use super::*;
use crate::models::{CommandLine, Panel};
use crate::{
    menu::{Heading, Menu, MenuCommand},
    models::{ChoiceTarget, MenuChoice, MenuDialog},
    packages::{Elevation, Manager, UpdatePlan},
};
use std::time::Duration;

mod submitted;

fn app(temp: &tempfile::TempDir) -> App {
    let panel = Panel::new(temp.path().to_path_buf()).expect("panel");
    App::with_panels(panel.clone(), panel)
}

#[test]
fn timing_records_processing_and_rendering_without_input_idle_time() {
    let temp = tempfile::TempDir::new().expect("tempdir");
    let mut app = app(&temp);
    let input_delivered = Instant::now() + Duration::from_secs(600);
    Pending::start(&app, &Event::Resize(80, 24), input_delivered).finish(
        &mut app.performance,
        input_delivered + Duration::from_millis(8),
        input_delivered + Duration::from_millis(10),
    );
    let report = app.performance.report().join("\n");
    assert!(report.contains("10.000"));
    assert!(report.contains("2.000"));
    assert!(!report.contains("600000"));
}

#[test]
fn typed_and_pasted_payloads_never_become_action_labels() {
    let temp = tempfile::TempDir::new().expect("tempdir");
    let mut app = app(&temp);
    app.set_overlay(Overlay::CommandLine(CommandLine::default()));
    assert_eq!(
        label(&app, &Event::Paste("private password".to_owned())),
        "CommandLine/Paste"
    );
    assert_eq!(command_label("echo private-password"), "Command/Shell");
    assert_eq!(command_label("cd private-path"), "Command/Cd");
    assert_eq!(
        command_label("codex private-prompt"),
        if cfg!(windows) {
            "Command/NewConsole"
        } else {
            "Command/Shell"
        }
    );
}

#[test]
fn submitted_commands_never_persist_parameters_or_invalid_input() {
    for (text, expected) in [
        ("pwd", "Command/Pwd"),
        ("sort size", "Command/Sort"),
        ("bookmark set 3", "Command/BookmarkSet"),
        ("bookmark go 3", "Command/BookmarkGo"),
        ("sort private-password", "Command/Invalid"),
        ("  ", "Command/Empty"),
    ] {
        assert_eq!(command_label(text), expected);
    }
}

#[test]
fn modal_paste_labels_identify_the_context_without_recording_private_text() {
    use crate::models::{ApplyPrompt, ArchiveMenu, DescribePrompt};
    let temp = tempfile::TempDir::new().unwrap();
    let mut app = app(&temp);
    let description = DescribePrompt::new("private name".into(), "private text", vec![]);
    for (overlay, expected) in [
        (Overlay::ArchivePrompt(Box::default()), "Archive"),
        (Overlay::ExtractPrompt(Box::default()), "Extract"),
        (
            Overlay::ArchiveCommands(ArchiveMenu::default()),
            "ArchiveCommands",
        ),
        (Overlay::Attributes(Box::default()), "Attributes"),
        (
            Overlay::ApplyCommand(ApplyPrompt::default()),
            "ApplyCommand",
        ),
        (Overlay::Describe(description), "Describe"),
        (Overlay::FindFile(Box::default()), "FindFile"),
        (Overlay::FindFolder(Box::default()), "FindFolder"),
        (Overlay::Menu(Menu::open(Heading::Commands)), "Menu"),
        (Overlay::Settings(Box::default()), "Settings"),
    ] {
        app.set_overlay(overlay);
        assert_eq!(
            label(&app, &Event::Paste("private payload".into())),
            format!("{expected}/Paste")
        );
    }
}

/// The Language servers dialog has its own label, like every other dialog
/// with a name, rather than the generic `Dialog`.
#[test]
fn the_language_servers_dialog_labels_its_own_actions() {
    let machine = crate::test_support::lsp_machine::LspMachine::new();
    let app = machine.opened();
    assert_eq!(
        label(&app, &Event::Paste("private payload".into())),
        "LspEditor/Paste"
    );
}

#[test]
fn mouse_and_focus_events_never_store_screen_coordinates() {
    use crossterm::event::{KeyModifiers, MouseEvent, MouseEventKind};
    let temp = tempfile::TempDir::new().expect("tempdir");
    let app = app(&temp);
    let mouse = Event::Mouse(MouseEvent {
        kind: MouseEventKind::ScrollDown,
        column: 42,
        row: 37,
        modifiers: KeyModifiers::NONE,
    });
    assert_eq!(label(&app, &mouse), "Panels/Mouse(ScrollDown)");
    assert_eq!(label(&app, &Event::FocusGained), "FocusGained");
    assert_eq!(label(&app, &Event::FocusLost), "FocusLost");
}

#[test]
fn keys_in_the_task_list_are_labelled_by_it() {
    let temp = tempfile::TempDir::new().expect("tempdir");
    let mut app = app(&temp);
    app.set_overlay(Overlay::Tasks(Box::default()));
    let key = Event::Key(KeyEvent::new(
        KeyCode::F(5),
        crossterm::event::KeyModifiers::NONE,
    ));
    assert_eq!(label(&app, &key), "Tasks/KeyModifiers(0x0)/F(5)");
}
