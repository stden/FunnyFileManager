//! Purpose: unit tests for the form's tab order, the controls it skips, and
//! the help each control shows.

use super::*;

#[test]
fn an_existing_servers_form_skips_its_language_in_tab_order() {
    let mut editor = editor();
    editor
        .open_form(&FormTarget::Existing(LanguageId::from_key("python")))
        .expect("opens");
    let form = form(&mut editor);
    assert_eq!(form.focus(), FormField::Command);
    let visited: Vec<FormField> = (0..6)
        .map(|_| {
            form.step_focus(Direction::Next);
            form.focus()
        })
        .collect();
    assert_eq!(
        visited,
        [
            FormField::Arguments,
            FormField::Extensions,
            FormField::Enabled,
            FormField::Ok,
            FormField::Cancel,
            FormField::Command
        ]
    );
    form.step_focus(Direction::Previous);
    assert_eq!(form.focus(), FormField::Cancel);
}

#[test]
fn an_existing_servers_language_is_shown_but_not_editable() {
    let mut editor = editor();
    editor
        .open_form(&FormTarget::Existing(LanguageId::from_key("python")))
        .expect("opens");
    let form = form(&mut editor);
    form.focus_on(FormField::Language);
    assert_eq!(
        form.focus(),
        FormField::Command,
        "the language cannot be renamed"
    );
    assert!(!FormField::Language.allows(form.target()));
    let text = |field| form.line(field).map(crate::lineedit::LineEditor::text);
    assert_eq!(text(FormField::Language), Some("python"));
    assert_eq!(text(FormField::Extensions), Some(".py .pyi"));
    assert_eq!(text(FormField::Enabled), None);
}

#[test]
fn a_new_servers_form_starts_on_its_language() {
    let mut editor = editor();
    editor.open_form(&FormTarget::New).expect("opens");
    let form = form(&mut editor);
    assert_eq!(form.focus(), FormField::Language);
    assert_eq!(form.target(), &FormTarget::New);
    assert_eq!(form.enabled(), Enablement::Enabled);
    assert_eq!(form.help(), "The LSP languageId: a-z 0-9 - _ . +");
    form.step_focus(Direction::Previous);
    assert_eq!(form.focus(), FormField::Cancel);
}

#[test]
fn every_control_has_its_help_and_the_text_ones_complete() {
    for field in FormField::ORDER {
        assert!(!field.help().is_empty());
        assert_eq!(
            field.completed().is_some(),
            field.is_text() && field != FormField::Language
        );
    }
    assert_eq!(
        FormField::Arguments.completed(),
        Some(crate::complete::servers::CompletedField::Arguments)
    );
    let mut editor = editor();
    editor.open_form(&FormTarget::New).expect("opens");
    let form = form(&mut editor);
    form.set_completion(Some(Suggestions::passive(menu(&["x"]))));
    assert!(form.completion().is_some());
    form.completion_mut().expect("a list").step(Direction::Next);
    assert_eq!(
        form.completion().map(Suggestions::focus),
        Some(ListFocus::List)
    );
    form.focus_on(FormField::Command);
    assert!(form.completion().is_none(), "moving focus closes the list");
}
