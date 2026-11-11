//! Purpose: unit tests for what OK accepts and refuses, and the values it
//! writes into the draft.

use super::*;

/// OK refuses the first field that does not parse, puts the cursor there and
/// says why; the draft does not change.
#[test]
fn a_refused_ok_focuses_the_field_and_says_why() {
    let mut editor = editor();
    editor
        .open_form(&FormTarget::Existing(LanguageId::from_key("python")))
        .expect("opens");
    type_into(&mut editor, FormField::Arguments, "\"unclosed");
    form(&mut editor).focus_on(FormField::Ok);
    assert_eq!(editor.accept_form(), None);
    let form = form(&mut editor);
    assert_eq!(form.focus(), FormField::Arguments);
    assert_eq!(form.refusal(), Some(&ValueError::QuoteNotClosed));
    assert_eq!(form.help(), "A quote is not closed.");
    form.step_focus(Direction::Next);
    assert_eq!(form.refusal(), None, "moving on forgets the refusal");
    assert!(!editor.dirty());
}

#[test]
fn ok_writes_the_four_fields_and_keeps_the_rest() {
    let mut editor = editor();
    editor
        .open_form(&FormTarget::Existing(LanguageId::from_key("python")))
        .expect("opens");
    type_into(&mut editor, FormField::Command, " /opt/pylsp ");
    type_into(&mut editor, FormField::Arguments, "--x=\"a b\" -v");
    type_into(&mut editor, FormField::Extensions, "py, .PYW");
    form(&mut editor).toggle_enabled();
    assert_eq!(editor.accept_form(), Some(LanguageId::from_key("python")));
    assert_eq!(editor.mode(), &Mode::Browse);
    let python = &editor.draft().0["python"];
    assert_eq!(python.command, "/opt/pylsp");
    assert_eq!(python.args, ["--x=a b", "-v"]);
    assert_eq!(python.extensions, ["py", "PYW"]);
    assert_eq!(python.enabled, Enablement::Disabled);
    assert_eq!(python.install, LspSettings::catalog().0["python"].install);
}

#[test]
fn a_new_server_needs_a_free_language_and_is_written_whole() {
    let mut editor = editor();
    editor.open_form(&FormTarget::New).expect("opens");
    type_into(&mut editor, FormField::Language, "python");
    type_into(&mut editor, FormField::Command, "x");
    assert_eq!(editor.accept_form(), None);
    assert_eq!(
        form(&mut editor).refusal(),
        Some(&ValueError::LanguageTaken("python".to_owned()))
    );
    type_into(&mut editor, FormField::Language, "zig");
    type_into(&mut editor, FormField::Command, "zls");
    assert_eq!(editor.accept_form(), Some(LanguageId::from_key("zig")));
    assert_eq!(editor.selected().map(LanguageId::as_str), Some("zig"));
    let zig = &editor.draft().0["zig"];
    assert_eq!(zig.command, "zls");
    assert_eq!(zig.extensions.len(), 0);
    assert_eq!(zig.root_markers, [".git"]);
    assert_eq!(zig.install, None);
}

/// A one-line field cannot show a control character, so the form does not
/// open on a server loaded with one; the notice names the field.
#[test]
fn the_form_refuses_to_open_on_a_loaded_control_character() {
    let loaded: LspSettings =
        serde_json::from_value(json!({"python": {"args": ["a\u{1b}b"]}})).expect("valid");
    let mut editor = LspEditor::new(Access::ReadOnly(ReadOnlyReason::NoSettingsFile), loaded);
    let refused = editor
        .open_form(&FormTarget::Existing(LanguageId::from_key("python")))
        .expect_err("refused");
    assert_eq!(
        refused,
        Notice::Error(
            "lsp.python.args holds a control character; fix settings.yaml by hand, or press \
             Del to reset a catalog server."
                .to_owned()
        )
    );
    assert_eq!(editor.mode(), &Mode::Browse);
    let server = &editor.draft().0["python"];
    assert_eq!(
        ServerForm::for_server(&LanguageId::from_key("python"), server),
        Err(OwnedField::Args)
    );
}

/// Only `parse` makes an `Accepted`, so what it lays over a server passed
/// the newtypes; the recipe and the root markers stay the server's own.
#[test]
fn an_accepted_form_lays_its_fields_over_a_server() {
    let mut form = ServerForm::new_server();
    let typed = [
        (FormField::Language, "mine"),
        (FormField::Command, " m "),
        (FormField::Arguments, "-v"),
        (FormField::Extensions, ".mn"),
    ];
    for (field, text) in typed {
        form.focus_on(field);
        form.focused_line_mut()
            .expect("a text field")
            .set_split(text, "");
    }
    form.toggle_enabled();
    let accepted = form.parse(|_| false).expect("every field parses");
    assert_eq!(accepted.language(), &LanguageId::from_key("mine"));
    assert_eq!(accepted.target(), &FormTarget::New);
    let server = accepted.new_server();
    assert_eq!(server.enabled, Enablement::Disabled);
    assert_eq!(server.command, "m", "trimmed by ServerCommand");
    assert_eq!(server.extensions, ["mn"], "the dot dropped by Extension");
    assert_eq!(server.root_markers, [".git"]);
    assert_eq!(server.install, None);
    let over = accepted.applied_to(LspSettings::catalog().0["python"].clone());
    assert_eq!(over.command, "m");
    assert!(
        over.install.is_some(),
        "the recipe is not the form's to change"
    );
}

#[test]
fn a_refusal_moves_the_cursor_and_closes_the_list() {
    let mut form = ServerForm::new_server();
    form.set_completion(Some(Suggestions::passive(menu(&["x"]))));
    form.refuse(FormField::Command, ValueError::CommandEmpty);
    assert_eq!(form.focus(), FormField::Command);
    assert!(form.completion().is_none());
    assert_eq!(form.help(), "Enter a command.");
}

/// A click that moves the cursor forgets the last refusal, as Tab does, so
/// the help row describes the field the cursor is on.
#[test]
fn moving_the_cursor_by_click_forgets_the_refusal() {
    let mut form = ServerForm::new_server();
    form.refuse(FormField::Command, ValueError::CommandEmpty);
    form.focus_on(FormField::Command);
    assert!(form.refusal().is_some(), "staying keeps it");
    form.focus_on(FormField::Arguments);
    assert_eq!(form.refusal(), None);
    assert_eq!(form.help(), FormField::Arguments.help());
}
