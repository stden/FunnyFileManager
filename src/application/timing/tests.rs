use super::*;
use crate::models::{CommandLine, Panel};
use crate::{
    menu::{Heading, Menu, MenuCommand},
    models::{ChoiceTarget, MenuChoice, MenuDialog},
    packages::{Manager, UpdatePlan},
};
use std::time::Duration;

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
    let description = DescribePrompt::new("private name".into(), "private text".into(), vec![]);
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
fn submitted_menu_operations_have_distinct_payload_free_names() {
    let temp = tempfile::TempDir::new().expect("tempdir");
    let mut app = app(&temp);
    let mut menu = Menu::open(Heading::Commands);
    menu.row = menu
        .entries()
        .iter()
        .position(|entry| entry.menu_command() == Some(MenuCommand::UpdatePackages))
        .expect("update row");
    app.set_overlay(Overlay::Menu(menu));
    assert_eq!(
        key_label(&app, KeyEvent::from(KeyCode::Enter)),
        "Menu/UpdatePackages"
    );
}

#[test]
fn submitted_package_update_excludes_paths_and_dialog_text() {
    let temp = tempfile::TempDir::new().expect("tempdir");
    let mut app = app(&temp);
    app.set_overlay(Overlay::MenuDialog(Box::new(MenuDialog::Choices {
        title: "private title".to_owned(),
        selected: 0,
        rows: vec![MenuChoice {
            label: "private text".to_owned(),
            target: ChoiceTarget::UpdatePackages(UpdatePlan {
                manager: Manager::Apt,
                executable: temp.path().join("private-path"),
            }),
        }],
    })));
    assert_eq!(
        key_label(&app, KeyEvent::from(KeyCode::Enter)),
        "PackageUpdate/apt-get"
    );
}

#[test]
fn submitted_dialogs_without_a_named_operation_keep_the_generic_label() {
    use crate::lineedit::LineEditor;
    let temp = tempfile::TempDir::new().expect("tempdir");
    let mut app = app(&temp);
    let enter = |app: &App| key_label(app, KeyEvent::from(KeyCode::Enter));
    app.set_overlay(Overlay::MenuDialog(Box::new(MenuDialog::Choices {
        title: "private title".to_owned(),
        selected: 0,
        rows: vec![MenuChoice {
            label: "private text".to_owned(),
            target: ChoiceTarget::Directory(temp.path().join("private-path")),
        }],
    })));
    let directory = enter(&app);
    assert!(directory.starts_with("Dialog/"), "{directory}");
    assert!(!directory.contains("private"), "{directory}");

    app.set_overlay(Overlay::MenuDialog(Box::new(MenuDialog::Prompt {
        command: MenuCommand::Filter,
        title: "private title".to_owned(),
        caption: "private caption".to_owned(),
        input: LineEditor::new("private-mask"),
    })));
    assert_eq!(enter(&app), directory);
}

#[test]
fn wipe_confirmation_labels_never_carry_the_paths_being_wiped() {
    let temp = tempfile::TempDir::new().expect("tempdir");
    let mut app = app(&temp);
    app.set_overlay(Overlay::WipeConfirm(vec![temp.path().join("private-path")]));
    assert_eq!(
        label(&app, &Event::Paste("private payload".into())),
        "Wipe/Paste"
    );
}
