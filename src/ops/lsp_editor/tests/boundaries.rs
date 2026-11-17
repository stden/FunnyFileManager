//! Purpose: Stale dialog actions — a removed language or server, an empty
//! draft, a field with nothing to accept — leave the draft and the form alone.

use super::*;
use crate::models::lsp_editor::FormField;

#[test]
fn checking_a_removed_language_keeps_the_draft_and_notices() {
    let machine = Machine::new();
    let mut app = machine.opened();
    let directory = app.left_panel.path.clone();
    let probe = app.live.lsp_probe.clone();
    let baseline = editor(&app).draft().clone();

    check_one(
        editor_mut(&mut app),
        LanguageId::from_key("missing-language"),
        &directory,
        &probe,
    );

    assert_eq!(editor(&app).draft(), &baseline);
    assert!(editor(&app).notice().is_none());
}

#[test]
fn deleting_from_an_empty_draft_does_nothing() {
    let machine = Machine::new();
    let mut app = machine.app();
    let dialog = LspEditor::new(
        Access::Writable {
            path: machine.settings(),
        },
        LspSettings(std::collections::BTreeMap::new()),
    );
    app.set_overlay(Overlay::LspEditor(Box::new(dialog)));

    delete_selected(&mut app);

    assert!(editor(&app).draft().0.is_empty());
    assert!(editor(&app).notice().is_none());
}

#[test]
fn opening_a_removed_server_reports_it_and_preserves_browse_mode() {
    let machine = Machine::new();
    let mut app = machine.opened();

    open_form(
        &mut app,
        &FormTarget::Existing(LanguageId::from_key("missing-language")),
    );

    assert!(matches!(editor(&app).mode(), Mode::Browse));
    assert_eq!(
        editor(&app).notice(),
        Some(&Notice::Error(
            "missing-language is not in the list".to_owned()
        ))
    );
}

#[test]
fn accepting_without_suggestions_or_on_a_checkbox_changes_no_text() {
    let machine = Machine::new();
    let mut app = machine.opened();
    open_form(&mut app, &FormTarget::New);
    let form = editor_mut(&mut app).form_mut().expect("form");
    form.focus_on(FormField::Command);
    form.focused_line_mut()
        .expect("command")
        .set_text("unchanged");
    accept_suggestion(&mut app);
    editor_mut(&mut app)
        .form_mut()
        .expect("form")
        .focus_on(FormField::Enabled);
    accept_suggestion(&mut app);

    let form = editor_mut(&mut app).form_mut().expect("form");
    assert_eq!(
        form.line(FormField::Command).expect("command").text(),
        "unchanged"
    );
    assert!(form.completion().is_none());
}
