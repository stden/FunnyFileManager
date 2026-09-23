//! Purpose: Consume editor LSP popup keys and delegate selected definitions to operations.

use crate::models::{lsp::Dialog, App};
use crossterm::event::KeyCode;

pub(super) fn process(app: &mut App, code: KeyCode) -> bool {
    let Some(editor) = app.get_editor_mut() else {
        return false;
    };
    let Some(mut dialog) = editor.lsp.popup.take() else {
        return false;
    };
    match code {
        KeyCode::Esc => return true,
        KeyCode::Enter => {
            accept(app, dialog);
            return true;
        }
        KeyCode::Up => navigate(&mut dialog, false),
        KeyCode::Down => navigate(&mut dialog, true),
        _ => {} // The popup consumes unsupported keys without editing the document.
    }
    editor.lsp.popup = Some(dialog);
    true
}

fn navigate(dialog: &mut Dialog, next: bool) {
    match dialog {
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

fn accept(app: &mut App, dialog: Dialog) {
    match dialog {
        Dialog::Completion {
            query,
            items,
            selected,
        } => {
            if let (Some(item), Some(editor)) = (items.get(selected), app.get_editor_mut()) {
                if let Err(message) = editor.complete_lsp(query, item) {
                    editor.notice = Some(message.into());
                }
            }
        }
        Dialog::Definitions { items, selected } => {
            if let Some(target) = items.get(selected) {
                crate::ops::editor_lsp::definition(app, target);
            }
        }
        Dialog::Hover { .. } => {}
    }
}

#[cfg(test)]
mod tests;
