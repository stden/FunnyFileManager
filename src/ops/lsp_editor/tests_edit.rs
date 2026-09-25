//! Purpose: Pin the dialog's edits — toggle, Del, the form's OK, F5 — and
//! when the suggestion list opens, refilters, closes and fills the field.

use crate::config::{Enablement, LanguageId};
use crate::menu::SettingsPage;
use crate::models::lsp_editor::{FormField, FormTarget, ListFocus, Notice};
use crate::models::App;
use crate::ops::lsp_editor::{
    accept_form, accept_suggestion, delete_selected, open_form, recheck, refresh_completion,
    toggle, Trigger,
};
use crate::test_support::lsp_machine::{editor, editor_mut, select, LspMachine as Machine};

/// Put `text` in `field` of the open form, caret at the end.
fn type_into(app: &mut App, field: FormField, text: &str) {
    let form = editor_mut(app).form_mut().expect("form");
    form.focus_on(field);
    form.focused_line_mut().expect("line").set_text(text);
}

#[test]
fn space_and_del_change_only_the_draft_and_say_what_they_did() {
    let machine = Machine::new();
    let mut app = machine.opened();
    select(&mut app, "python");
    toggle(&mut app);
    assert_eq!(
        editor(&app).draft().0["python"].enabled,
        Enablement::Disabled
    );
    delete_selected(&mut app);
    assert_eq!(
        editor(&app).notice(),
        Some(&Notice::Info("python: reset to the catalog.".to_owned()))
    );
    assert!(!editor(&app).dirty());
    assert!(
        !machine.settings().exists(),
        "nothing reaches the file before F2"
    );
}

#[test]
fn del_removes_an_added_server() {
    let machine = Machine::new();
    machine.write_settings("lsp:\n  mine: {command: m, extensions: [mn]}\n");
    let mut app = machine.opened();
    select(&mut app, "mine");
    delete_selected(&mut app);
    assert!(!editor(&app).draft().0.contains_key("mine"));
    assert_eq!(
        editor(&app).notice(),
        Some(&Notice::Info("mine: removed.".to_owned()))
    );
}

/// Del checks the server it reset again: the file's command is on the fake
/// `PATH`, the catalog's is not, so a reset server is no longer ready.
#[test]
fn del_checks_the_reset_server_again() {
    let machine = Machine::new();
    machine.install_runtimes();
    let tool = machine.bin().join(crate::lsp::program_file("my-pylsp"));
    crate::test_support::write_executable(&tool, b"tool").expect("tool");
    machine.write_settings("lsp:\n  python: {command: my-pylsp}\n");
    let mut app = machine.opened();
    let python = LanguageId::from_key("python");
    let tag = |app: &App| {
        editor(app)
            .availability(&python)
            .map(crate::lsp::Availability::tag)
    };
    assert_eq!(tag(&app), Some("ready"));
    select(&mut app, "python");
    delete_selected(&mut app);
    assert_eq!(tag(&app), Some("auto"));
}

/// OK rechecks the server it changed: a command put on the fake `PATH` is
/// found at once.
#[test]
fn ok_checks_the_changed_server_again() {
    let machine = Machine::new();
    let tool = machine.bin().join(crate::lsp::program_file("my-pylsp"));
    crate::test_support::write_executable(&tool, b"tool").expect("tool");
    let mut app = machine.opened();
    open_form(
        &mut app,
        &FormTarget::Existing(LanguageId::from_key("python")),
    );
    type_into(&mut app, FormField::Command, "my-pylsp");
    accept_form(&mut app);
    let found = editor(&app)
        .availability(&LanguageId::from_key("python"))
        .expect("checked");
    assert_eq!(found.tag(), "ready");
    assert!(matches!(found.resolved(), Some(resolved) if resolved.executable == tool));
}

#[test]
fn f5_checks_every_server_again() {
    let machine = Machine::new();
    machine.install_runtimes();
    let mut app = machine.opened();
    assert_eq!(
        editor(&app)
            .availability(&LanguageId::from_key("python"))
            .map(crate::lsp::Availability::tag),
        Some("auto")
    );
    let pylsp = machine.bin().join(crate::lsp::program_file("pylsp"));
    crate::test_support::write_executable(&pylsp, b"pylsp").expect("program");
    recheck(&mut app);
    assert_eq!(
        editor(&app)
            .availability(&LanguageId::from_key("python"))
            .map(crate::lsp::Availability::tag),
        Some("ready")
    );
    assert_eq!(
        editor(&app).notice(),
        Some(&Notice::Info("Checked every server again.".to_owned()))
    );
}

#[test]
fn a_read_only_dialog_opens_no_form() {
    let machine = Machine::new();
    machine.write_settings("[]");
    let mut app = machine.opened();
    open_form(&mut app, &FormTarget::New);
    assert!(editor_mut(&mut app).form_mut().is_none());
    assert_eq!(
        editor(&app).notice(),
        Some(&Notice::Error(
            "Read only: settings.yaml does not parse: expected a mapping of settings.".to_owned()
        ))
    );
}

#[path = "tests_edit_suggestions.rs"]
mod tests_edit_suggestions;
