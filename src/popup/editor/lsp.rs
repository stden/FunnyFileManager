//! Purpose: Consume editor LSP popup keys and delegate selected definitions to operations.

use crate::models::{
    lsp::{is_identifier_char, Dialog},
    App,
};
use crossterm::event::{KeyCode, KeyModifiers};

#[cfg(test)]
pub(super) fn process(app: &mut App, code: KeyCode) -> bool {
    process_key(app, code, KeyModifiers::NONE).is_some()
}

pub(super) fn process_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> Option<bool> {
    let editor = app.get_editor_mut()?;
    let dialog = editor.lsp.popup.take()?;
    if let Dialog::Completion {
        query,
        items,
        selected,
    } = dialog
    {
        return process_completion(app, query, items, selected, code, modifiers);
    }
    process_modal_dialog(app, dialog, code);
    Some(true)
}

fn process_completion(
    app: &mut App,
    query: crate::lsp::types::Query,
    items: Vec<crate::lsp::features::Completion>,
    mut selected: usize,
    code: KeyCode,
    modifiers: KeyModifiers,
) -> Option<bool> {
    let plain = !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);
    match code {
        KeyCode::Esc => {
            dismiss_completion(app, true);
            return Some(true);
        }
        KeyCode::Enter | KeyCode::Tab => {
            apply_completion(app, query, &items, selected, code == KeyCode::Tab);
            return Some(true);
        }
        KeyCode::Up => move_selection(&mut selected, items.len(), false),
        KeyCode::Down => move_selection(&mut selected, items.len(), true),
        KeyCode::PageUp => selected = selected.saturating_sub(8),
        KeyCode::PageDown => {
            selected = selected
                .saturating_add(8)
                .min(items.len().saturating_sub(1));
        }
        _ => return edit_completion(app, query, items, code, plain),
    }
    if let Some(editor) = app.get_editor_mut() {
        editor.lsp.popup = Some(Dialog::Completion {
            query,
            items,
            selected,
        });
    }
    Some(true)
}

fn edit_completion(
    app: &mut App,
    query: crate::lsp::types::Query,
    items: Vec<crate::lsp::features::Completion>,
    code: KeyCode,
    plain: bool,
) -> Option<bool> {
    let editor = app.get_editor_mut()?;
    match code {
        KeyCode::Char(ch) if plain && is_identifier_char(ch) => {
            editor.notice = None;
            editor.insert_char(ch);
            editor.maybe_trigger_completion(ch);
            editor.update_live_completion(query, items);
            Some(true)
        }
        KeyCode::Backspace if plain => {
            editor.notice = None;
            let had_prefix = !editor.completion_prefix().is_empty();
            editor.backspace();
            if !had_prefix || editor.completion_prefix().is_empty() {
                editor.lsp.completions = None;
            } else {
                editor.update_live_completion(query, items);
            }
            Some(true)
        }
        _ => {
            dismiss_completion(app, false);
            None
        }
    }
}

fn dismiss_completion(app: &mut App, cancel_auto: bool) {
    if let Some(editor) = app.get_editor_mut() {
        editor.lsp.completions = None;
        if cancel_auto {
            editor.lsp.auto_complete = false;
        }
    }
}

fn apply_completion(
    app: &mut App,
    query: crate::lsp::types::Query,
    items: &[crate::lsp::features::Completion],
    selected: usize,
    replace_suffix: bool,
) {
    let Some(item) = items.get(selected) else {
        return;
    };
    let Some(editor) = app.get_editor_mut() else {
        return;
    };
    let result = if replace_suffix {
        editor.complete_lsp_replace(query, item)
    } else {
        editor.complete_lsp(query, item)
    };
    if let Err(message) = result {
        editor.notice = Some(message.into());
    }
}

fn process_modal_dialog(app: &mut App, mut dialog: Dialog, code: KeyCode) {
    match code {
        KeyCode::Esc => return,
        KeyCode::Enter => {
            accept_modal(app, dialog);
            return;
        }
        KeyCode::Up => navigate(&mut dialog, false),
        KeyCode::Down => navigate(&mut dialog, true),
        _ => {}
    }
    if let Some(editor) = app.get_editor_mut() {
        editor.lsp.popup = Some(dialog);
    }
}

fn navigate(dialog: &mut Dialog, next: bool) {
    match dialog {
        Dialog::Trust { allow, .. } => *allow = next,
        Dialog::Completion {
            selected, items, ..
        } => move_selection(selected, items.len(), next),
        Dialog::Definitions { selected, items } => move_selection(selected, items.len(), next),
        Dialog::Hover { scroll, .. } => {
            *scroll = if next {
                scroll.saturating_add(1)
            } else {
                scroll.saturating_sub(1)
            }
        }
    }
}

fn move_selection(selected: &mut usize, len: usize, next: bool) {
    *selected = if next {
        selected.saturating_add(1).min(len.saturating_sub(1))
    } else {
        selected.saturating_sub(1)
    };
}

fn accept_modal(app: &mut App, dialog: Dialog) {
    match dialog {
        Dialog::Trust { allow, .. } => {
            if allow {
                if let Some(editor) = app.get_editor_mut() {
                    editor.lsp.intent = crate::models::lsp::Intent::Trust;
                }
            }
        }
        Dialog::Definitions { items, selected } => {
            if let Some(target) = items.get(selected) {
                crate::ops::editor_lsp::definition(app, target);
            }
        }
        Dialog::Completion { .. } | Dialog::Hover { .. } => {}
    }
}

#[cfg(test)]
mod tests;
