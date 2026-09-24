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

fn field_text(app: &mut App, field: FormField) -> String {
    let form = editor_mut(app).form_mut().expect("form");
    form.line(field).expect("line").text().to_owned()
}

fn list_focus(app: &mut App) -> Option<ListFocus> {
    let form = editor_mut(app).form_mut().expect("form");
    form.completion()
        .map(crate::models::lsp_editor::Suggestions::focus)
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

/// With `AutoComplete` on, typing opens a passive list; off, it opens none.
#[test]
fn typing_opens_a_passive_list_only_while_autocomplete_is_on() {
    let machine = Machine::new();
    let mut app = machine.opened();
    open_form(
        &mut app,
        &FormTarget::Existing(LanguageId::from_key("python")),
    );
    type_into(&mut app, FormField::Command, "rust-an");
    refresh_completion(&mut app, Trigger::Typed);
    assert_eq!(list_focus(&mut app), Some(ListFocus::Field));

    app.settings
        .set(SettingsPage::AutoComplete, "off")
        .expect("valid");
    let form = editor_mut(&mut app).form_mut().expect("form");
    form.set_completion(None);
    refresh_completion(&mut app, Trigger::Typed);
    assert_eq!(list_focus(&mut app), None);
}

fn candidates(app: &mut App) -> Vec<String> {
    let form = editor_mut(app).form_mut().expect("form");
    form.completion().map_or_else(Vec::new, |list| {
        list.menu()
            .candidates()
            .iter()
            .map(|candidate| candidate.text.clone())
            .collect()
    })
}

fn python_form(machine: &Machine) -> App {
    let mut app = machine.opened();
    open_form(
        &mut app,
        &FormTarget::Existing(LanguageId::from_key("python")),
    );
    app
}

/// An open list follows every edit, whatever the setting, and closes when
/// nothing matches.
#[test]
fn an_open_list_follows_edits_and_closes_when_nothing_matches() {
    let machine = Machine::new();
    let mut app = python_form(&machine);
    app.settings
        .set(SettingsPage::AutoComplete, "off")
        .expect("valid");
    type_into(&mut app, FormField::Command, "vscode");
    refresh_completion(&mut app, Trigger::Requested);
    assert_eq!(list_focus(&mut app), Some(ListFocus::List));
    type_into(&mut app, FormField::Command, "vscode-css");
    refresh_completion(&mut app, Trigger::Typed);
    assert_eq!(candidates(&mut app), ["vscode-css-language-server"]);
    type_into(&mut app, FormField::Command, "vscode-qqq");
    refresh_completion(&mut app, Trigger::Typed);
    assert_eq!(list_focus(&mut app), None);
}

/// Typing opens nothing for an empty token or one its only match equals.
#[test]
fn typing_opens_no_list_that_would_offer_only_the_token() {
    let machine = Machine::new();
    let mut app = python_form(&machine);
    type_into(&mut app, FormField::Command, "pylsp");
    refresh_completion(&mut app, Trigger::Typed);
    assert_eq!(list_focus(&mut app), None);
    type_into(&mut app, FormField::Command, "");
    refresh_completion(&mut app, Trigger::Typed);
    assert_eq!(list_focus(&mut app), None);
}

#[test]
fn a_request_with_no_match_says_so_and_one_match_is_taken() {
    let machine = Machine::new();
    let mut app = python_form(&machine);
    type_into(&mut app, FormField::Command, "qqq");
    refresh_completion(&mut app, Trigger::Requested);
    assert_eq!(
        editor(&app).notice(),
        Some(&Notice::Info("No suggestions".to_owned()))
    );
    type_into(&mut app, FormField::Command, "jdt");
    refresh_completion(&mut app, Trigger::Requested);
    assert_eq!(field_text(&mut app, FormField::Command), "jdtls");
    assert_eq!(list_focus(&mut app), None, "one match is taken outright");
    editor_mut(&mut app)
        .form_mut()
        .expect("form")
        .focus_on(FormField::Enabled);
    editor_mut(&mut app).clear_notice();
    refresh_completion(&mut app, Trigger::Requested);
    assert_eq!(list_focus(&mut app), None, "a control without a list");
    assert_eq!(
        editor(&app).notice(),
        Some(&Notice::Info("No suggestions".to_owned())),
        "a request there still gets an answer"
    );
}

/// Typing in a field without suggestions, a new server's language, is
/// answered by nothing: "No suggestions" is for a request.
#[test]
fn typing_where_nothing_is_offered_says_nothing() {
    let machine = Machine::new();
    let mut app = machine.opened();
    open_form(&mut app, &FormTarget::New);
    type_into(&mut app, FormField::Language, "zig");
    refresh_completion(&mut app, Trigger::Typed);
    assert_eq!(editor(&app).notice(), None);
    assert_eq!(list_focus(&mut app), None);
}

#[test]
fn a_request_with_many_matches_opens_an_active_list_to_accept_from() {
    let machine = Machine::new();
    let mut app = python_form(&machine);
    type_into(&mut app, FormField::Arguments, "");
    refresh_completion(&mut app, Trigger::Requested);
    assert_eq!(list_focus(&mut app), Some(ListFocus::List));
    assert_eq!(candidates(&mut app), ["start", "--stdio"]);
    accept_suggestion(&mut app);
    assert_eq!(field_text(&mut app, FormField::Arguments), "start ");
    assert_eq!(list_focus(&mut app), None);
}
