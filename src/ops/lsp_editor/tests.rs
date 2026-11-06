//! Purpose: Pin how the dialog opens on the settings file, what F2 writes, and
//! that every refused or failed save leaves the file and the draft alone.

use std::{fs, io};

use super::save::save_with;
use super::*;
use crate::config::Enablement;
use crate::lsp::NotInstalled;
use crate::models::lsp_editor::Mode;
use crate::models::settings::Settings;

use crate::test_support::lsp_machine::{editor, editor_mut, LspMachine as Machine};

/// `text` as YAML, to compare with what the file holds.
fn yaml(text: &str) -> serde_norway::Value {
    serde_norway::from_str(text).expect("YAML")
}

#[path = "tests_edit.rs"]
mod edit;

mod boundaries;
mod opening;
mod saving;

#[test]
fn lsp_editor_operations_are_no_ops_when_overlay_is_inactive() {
    let mut app = Machine::new().app();
    recheck(&mut app);
    toggle(&mut app);
    delete_selected(&mut app);
    open_form(&mut app, &FormTarget::New);
    accept_form(&mut app);
    refresh_completion(&mut app, Trigger::Typed);
    accept_suggestion(&mut app);
    assert_eq!(save(&mut app), SaveOutcome::Refused);
}

#[test]
fn lsp_editor_guards_absent_languages_catalog_rows_and_inactive_completions() {
    let mut app = Machine::new().app();
    open(&mut app);
    refresh_completion(&mut app, Trigger::Typed);
    accept_suggestion(&mut app);
    delete_selected(&mut app);
    let probe = app.live.lsp_probe.clone();
    check_one(
        editor_mut(&mut app),
        LanguageId::from_key("absent-language"),
        Path::new("."),
        &probe,
    );
    open_form(
        &mut app,
        &FormTarget::Existing(LanguageId::from_key("absent-language")),
    );
    open_form(&mut app, &FormTarget::New);
    editor_mut(&mut app)
        .form_mut()
        .expect("form")
        .focus_on(crate::models::lsp_editor::FormField::Language);
    accept_suggestion(&mut app);
    editor_mut(&mut app)
        .form_mut()
        .expect("form")
        .focus_on(crate::models::lsp_editor::FormField::Command);
    accept_suggestion(&mut app);
}
