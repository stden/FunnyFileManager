//! Purpose: When the Language servers form's suggestion list opens, refilters
//! and closes, and what accepting a suggestion puts in the field.
//!
//! A keystroke never scans `PATH`: the program names come from the probe,
//! which answers from a finished background scan or not at all.

use crate::complete::servers::{self, CompletedField, Vocabulary};
use crate::config::LspSettings;
use crate::menu::SettingsPage;
use crate::models::lsp_editor::{Mode, Notice, ServerForm, Suggestions};
use crate::models::{App, Candidate, LspEditor, Menu, Overlay};

/// Why the list is being looked at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    /// A character, Backspace or Delete changed the field.
    Typed,
    /// Ctrl+Space, Ctrl+Down or Alt+Down asked for the list.
    Requested,
}

/// Whether typing opens a list on its own: the Options `AutoComplete` page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AutoOpen {
    On,
    Off,
}

/// Open, refilter or close the focused field's suggestion list.
///
/// An open list follows every edit, whatever the setting, and closes when
/// nothing matches. With no list open, typing opens a passive one only while
/// `AutoComplete` is on; a request opens an active one, accepts a single match
/// outright, or says there is nothing to offer.
pub fn refresh_completion(app: &mut App, trigger: Trigger) {
    let auto = if app.settings.enabled(SettingsPage::AutoComplete) {
        AutoOpen::On
    } else {
        AutoOpen::Off
    };
    let programs = app.live.lsp_probe.programs();
    let Overlay::LspEditor(editor) = &mut app.overlay else {
        return;
    };
    let offer = offer(editor, programs);
    let Some(form) = editor.form_mut() else {
        return;
    };
    let notice = match offer {
        Some((field, candidates, typed)) => {
            update(form, trigger, auto, field, Offer { candidates, typed })
        }
        None if trigger == Trigger::Requested => Some(no_suggestions()),
        None => None,
    };
    if let Some(notice) = notice {
        editor.set_notice(notice);
    }
}

/// The suggestions for the focused field, and its token as typed.
struct Offer {
    candidates: Vec<Candidate>,
    typed: String,
}

fn offer(
    editor: &LspEditor,
    programs: &[String],
) -> Option<(CompletedField, Vec<Candidate>, String)> {
    let Mode::Form(form) = editor.mode() else {
        return None;
    };
    let field = form.focus().completed()?;
    let line = form.line(form.focus())?;
    let vocabulary = Vocabulary {
        catalog: LspSettings::catalog(),
        draft: editor.draft(),
        programs,
    };
    Some((
        field,
        servers::candidates(field, line, &vocabulary),
        servers::typed(field, line),
    ))
}

fn no_suggestions() -> Notice {
    Notice::Info("No suggestions".to_owned())
}

fn update(
    form: &mut ServerForm,
    trigger: Trigger,
    auto: AutoOpen,
    field: CompletedField,
    offer: Offer,
) -> Option<Notice> {
    if trigger == Trigger::Requested {
        return request(form, field, offer.candidates);
    }
    let menu = Menu::new(offer.candidates);
    let refilled = form
        .completion()
        .map(|open| menu.clone().map(|menu| open.refilled(menu)));
    match (refilled, auto, menu) {
        (Some(refilled), _, _) => form.set_completion(refilled),
        (None, AutoOpen::On, Some(menu)) if worth_showing(&menu, &offer.typed) => {
            form.set_completion(Some(Suggestions::passive(menu)));
        }
        (None, _, _) => {}
    }
    None
}

/// A list typing may open: a token typed, and more to it than the token.
fn worth_showing(menu: &Menu, typed: &str) -> bool {
    let only_itself = menu.candidates().len() == 1 && menu.selection().text == typed;
    !typed.is_empty() && !only_itself
}

/// A request: nothing to offer, one match to take, or a list to choose from.
fn request(
    form: &mut ServerForm,
    field: CompletedField,
    candidates: Vec<Candidate>,
) -> Option<Notice> {
    let Some(menu) = Menu::new(candidates) else {
        form.set_completion(None);
        return Some(no_suggestions());
    };
    if let [only] = menu.candidates() {
        let text = only.text.clone();
        if let Some(line) = form.focused_line_mut() {
            servers::accept(field, line, &text);
        }
        form.set_completion(None);
        return None;
    }
    form.set_completion(Some(Suggestions::active(menu)));
    None
}

/// Enter on an active list: the highlighted suggestion into the field, and
/// the list closed.
pub fn accept_suggestion(app: &mut App) {
    let Overlay::LspEditor(editor) = &mut app.overlay else {
        return;
    };
    let Some(form) = editor.form_mut() else {
        return;
    };
    let Some(field) = form.focus().completed() else {
        return;
    };
    let Some(text) = form
        .completion()
        .map(|list| list.menu().selection().text.clone())
    else {
        return;
    };
    if let Some(line) = form.focused_line_mut() {
        servers::accept(field, line, &text);
    }
    form.set_completion(None);
}
