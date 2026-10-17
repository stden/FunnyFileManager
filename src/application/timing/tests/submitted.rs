//! Purpose: timing labels of submitted menu operations and dialogs — distinct names, never the paths or text they carried.

use super::*;

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
