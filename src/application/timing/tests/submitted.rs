//! Purpose: timing labels of submitted menu operations and dialogs — distinct names, never the paths or text they carried.

use super::*;
use crate::action::{FileCommand, PanelCommand, ToolCommand};

/// The drop-down `heading`, with the cursor on the row that runs `action`.
fn menu_at(heading: Heading, action: Action) -> Menu {
    let mut menu = Menu::open(heading);
    menu.row = menu
        .entries()
        .iter()
        .position(|entry| entry.action() == Some(action))
        .unwrap_or_else(|| panic!("{heading:?} has no {action:?} row"));
    menu
}

#[test]
fn submitted_menu_operations_have_distinct_payload_free_names() {
    let temp = tempfile::TempDir::new().expect("tempdir");
    let mut app = app(&temp);
    app.set_overlay(Overlay::Menu(menu_at(
        Heading::Commands,
        Action::Menu(MenuCommand::UpdatePackages),
    )));
    assert_eq!(
        key_label(&app, KeyEvent::from(KeyCode::Enter)),
        "Menu/UpdatePackages"
    );
}

/// A row with a key is named by the command it holds, whichever family
/// names it, as it was while every keyed row was a `MenuCommand`: the stats
/// screen keeps one line per menu function instead of one "Menu/Enter" line
/// for all of them.
#[test]
fn submitted_keyed_rows_are_named_by_their_command() {
    let temp = tempfile::TempDir::new().expect("tempdir");
    let mut app = app(&temp);
    for (heading, action, label) in [
        (Heading::Files, Action::File(FileCommand::Link), "Menu/Link"),
        (
            Heading::Left,
            Action::Panel(PanelCommand::Tree),
            "Menu/Tree",
        ),
        (
            Heading::Commands,
            Action::Tool(ToolCommand::FindFile),
            "Menu/FindFile",
        ),
    ] {
        app.set_overlay(Overlay::Menu(menu_at(heading, action)));
        assert_eq!(key_label(&app, KeyEvent::from(KeyCode::Enter)), label);
    }
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
                elevation: Elevation::Sudo,
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
        purpose: MenuPrompt::Filter,
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

#[test]
fn submitted_rows_running_a_panel_key_keep_the_generic_label() {
    let temp = tempfile::TempDir::new().expect("tempdir");
    let mut app = app(&temp);
    app.set_overlay(Overlay::Menu(menu_at(Heading::Left, Action::Refresh)));
    let label = key_label(&app, KeyEvent::from(KeyCode::Enter));
    assert!(label.starts_with("Menu/"), "{label}");
    assert!(!label.contains("Refresh"), "{label}");
}
