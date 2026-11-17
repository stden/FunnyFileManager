//! Purpose: When the suggestion list opens, refilters, closes and fills the field.

use super::*;

fn field_text(app: &mut App, field: FormField) -> String {
    let form = editor_mut(app).form_mut().expect("form");
    form.line(field).expect("line").text().to_owned()
}

fn list_focus(app: &mut App) -> Option<ListFocus> {
    let form = editor_mut(app).form_mut().expect("form");
    form.completion()
        .map(crate::models::lsp_editor::Suggestions::focus)
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
