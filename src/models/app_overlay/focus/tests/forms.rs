//! Purpose: Verify search, menu, language-server and PATH forms route paste to the focused field.

use super::{app, paste};
use crate::config::LspSettings;
use crate::lineedit::LineEditor;
use crate::menu::MenuCommand;
use crate::models::lsp_editor::{Access, FormField, FormTarget, Mode as LspMode};
use crate::models::{LspEditor, MenuDialog, Overlay};
use crate::path_env::{Mode as PathMode, PathEditor, Scope, Snapshot};
use tempfile::TempDir;

#[test]
fn find_file_paste_follows_the_active_field_and_skips_the_buttons() {
    let root = TempDir::new().unwrap();
    let mut app = app(&root, Overlay::FindFile(Box::default()));
    paste(&mut app, ".rs");
    let Overlay::FindFile(dialog) = &mut app.overlay else {
        panic!("the find dialog stays open");
    };
    dialog.tab(true);
    paste(&mut app, "needle");
    let Overlay::FindFile(dialog) = &mut app.overlay else {
        panic!("the find dialog stays open");
    };
    assert_eq!(dialog.mask.text(), "*.rs");
    assert_eq!(dialog.containing.text(), "needle");
    dialog.tab(true);
    assert!(app.focused_text_field().is_none());
}

#[test]
fn menu_prompts_accept_paste_and_choice_lists_do_not() {
    let root = TempDir::new().unwrap();
    let prompt = MenuDialog::Prompt {
        command: MenuCommand::FindFolder,
        title: "Find folder".into(),
        caption: "Folder:".into(),
        input: LineEditor::new("folder"),
    };
    let mut app = app(&root, Overlay::MenuDialog(Box::new(prompt)));
    paste(&mut app, "/child");
    let Overlay::MenuDialog(dialog) = &app.overlay else {
        panic!("the menu prompt stays open");
    };
    assert!(
        matches!(dialog.as_ref(), MenuDialog::Prompt { command, input, .. }
        if *command == MenuCommand::FindFolder && input.text() == "folder/child")
    );
    app.set_overlay(Overlay::MenuDialog(Box::new(MenuDialog::Choices {
        title: "Folders".into(),
        rows: Vec::new(),
        selected: 0,
    })));
    assert!(app.focused_text_field().is_none());
}

fn language_server_app(root: &TempDir) -> crate::models::App {
    let mut editor = LspEditor::new(
        Access::Writable {
            path: root.path().join("settings.yaml"),
        },
        LspSettings::default(),
    );
    editor.open_form(&FormTarget::New).unwrap();
    app(root, Overlay::LspEditor(Box::new(editor)))
}

#[test]
fn language_server_paste_edits_the_form_without_changing_the_draft() {
    let root = TempDir::new().unwrap();
    let mut app = language_server_app(&root);
    paste(&mut app, "custom");
    let Overlay::LspEditor(editor) = &mut app.overlay else {
        panic!("the language-server editor stays open");
    };
    assert!(!editor.dirty());
    let form = editor.form_mut().unwrap();
    assert_eq!(form.line(FormField::Language).unwrap().text(), "custom");
}

#[test]
fn language_server_buttons_and_browse_mode_offer_no_text_field() {
    let root = TempDir::new().unwrap();
    let mut app = language_server_app(&root);
    let Overlay::LspEditor(editor) = &mut app.overlay else {
        panic!("the editor is open")
    };
    let form = editor.form_mut().unwrap();
    form.focus_on(FormField::Enabled);
    assert!(app.focused_text_field().is_none());
    let Overlay::LspEditor(editor) = &mut app.overlay else {
        panic!("the editor stays open")
    };
    editor.set_mode(LspMode::Browse);
    assert!(app.focused_text_field().is_none());
}

#[test]
fn path_paste_edits_only_the_open_entry_without_changing_its_snapshot() {
    let root = TempDir::new().unwrap();
    let snapshot = Snapshot {
        value: String::new(),
        revision: "initial".into(),
        location: "fixture".into(),
    };
    let mut app = app(
        &root,
        Overlay::PathEditor(Box::new(PathEditor::new(Scope::User, snapshot))),
    );
    assert!(app.focused_text_field().is_none());
    let Overlay::PathEditor(editor) = &mut app.overlay else {
        panic!("the PATH editor is open")
    };
    editor.edit(None);
    paste(&mut app, "new/path");
    let Overlay::PathEditor(editor) = &app.overlay else {
        panic!("the PATH editor stays open")
    };
    let PathMode::Edit { input, .. } = &editor.mode else {
        panic!("the entry is being edited")
    };
    assert_eq!(input.text(), "new/path");
    assert!(editor.snapshot.value.is_empty());
    assert!(editor.entries.is_empty());
}
